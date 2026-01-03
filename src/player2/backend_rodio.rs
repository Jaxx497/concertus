#![allow(unused)]

use anyhow::Result;
use rodio::{
    decoder::builder::SeekMode, ChannelCount, Decoder, OutputStream, OutputStreamBuilder, Sink,
    Source,
};
use std::{
    fs::File,
    io::BufReader,
    num::NonZero,
    ops::Sub,
    path::PathBuf,
    sync::{
        mpsc::{Receiver, Sender},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::{
    domain::{QueueSong, SimpleSong},
    player2::{metrics::PlaybackMetrics, PlayerCommand, PlayerEvent},
    REFRESH_RATE,
};

pub struct PlayerBackend {
    pub sink: Sink,
    _stream: OutputStream,
}

pub struct PlayerCore {
    backend: PlayerBackend,
    commands: Receiver<PlayerCommand>,
    events: Sender<PlayerEvent>,
    metrics: Arc<PlaybackMetrics>,

    current: Option<Arc<SimpleSong>>,
    next: Option<Arc<QueueSong>>,
}

impl PlayerCore {
    pub fn spawn(
        commands: Receiver<PlayerCommand>,
        events: Sender<PlayerEvent>,
        metrics: Arc<PlaybackMetrics>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            let _stream = OutputStreamBuilder::open_default_stream().expect("Cannot open stream");
            let sink = Sink::connect_new(_stream.mixer());

            let mut core = PlayerCore {
                backend: PlayerBackend { sink, _stream },
                commands,
                events,
                metrics,

                current: None,
                next: None,
            };

            core.run();
        })
    }

    fn run(&mut self) {
        loop {
            self.process_commands();
            self.check_track_end();
            self.update_metrics();
            thread::sleep(REFRESH_RATE);
        }
    }

    fn process_commands(&mut self) {
        while let Ok(cmd) = self.commands.try_recv() {
            match cmd {
                PlayerCommand::Play(s) => self.play_song(&s),
                PlayerCommand::SetNext(s) => self.set_next(s),
                PlayerCommand::TogglePlayback => self.toggle_playback(),
                PlayerCommand::Stop => self.stop(),
                PlayerCommand::SeekForward(x) => self.seek_forward(x),
                PlayerCommand::SeekBack(x) => self.seek_back(x),
            }
        }
    }

    fn check_track_end(&mut self) {
        if self.sink_is_empty() && self.current.is_some() {
            self.current = None;
            let _ = self.events.send(PlayerEvent::PlaybackStopped);
        }
    }

    fn update_metrics(&mut self) {
        if self.current.is_some() {
            self.metrics.set_elapsed(self.backend.sink.get_pos());
        }
    }
}

impl PlayerCore {
    fn play_song(&mut self, song: &Arc<QueueSong>) {
        let source = match decode(song) {
            Ok(s) => s,
            Err(e) => {
                let _ = self.events.send(PlayerEvent::Error(e.to_string()));
                return;
            }
        };

        let tapped = TappedSource::new(source, Arc::clone(&self.metrics));

        let player = &self.backend.sink;

        player.clear();
        player.append(tapped);
        player.play();
        self.current = Some(Arc::clone(&song.meta));

        let _ = self
            .events
            .send(PlayerEvent::TrackStarted(Arc::clone(&song)));
    }

    fn set_next(&mut self, song: Option<Arc<QueueSong>>) {
        self.next = song
    }

    fn toggle_playback(&mut self) {
        if self.backend.sink.empty() {
            return;
        }
        match self.backend.sink.is_paused() {
            true => self.backend.sink.play(),
            false => self.backend.sink.pause(),
        }
    }

    pub(crate) fn stop(&mut self) {
        self.backend.sink.stop();
        self.current = None;
        self.metrics.reset();
        let _ = self.events.send(PlayerEvent::PlaybackStopped);
    }

    pub(crate) fn seek_forward(&mut self, secs: u64) {
        if let Some(now_playing) = &self.current {
            let duration = now_playing.duration;
            let elapsed = self.metrics.get_elapsed();

            let new_time = Duration::from_secs(secs) + elapsed;
            if new_time < duration {
                if let Err(e) = self.backend.sink.try_seek(new_time) {
                    let _ = self.events.send(PlayerEvent::Error(e.to_string()));
                };
            } else {
                self.stop();
            }
        }
    }

    pub(crate) fn seek_back(&mut self, secs: u64) {
        if let Some(_) = &self.current {
            let elapsed = self.metrics.get_elapsed();

            if elapsed.as_secs_f32() > 5.0 {
                let _ = self
                    .backend
                    .sink
                    .try_seek(elapsed.sub(Duration::from_secs(secs)));
            } else {
                let _ = self.backend.sink.try_seek(Duration::ZERO);
            }
        }
    }

    pub(crate) fn sink_is_empty(&self) -> bool {
        self.backend.sink.empty()
    }
}

fn decode(song: &Arc<QueueSong>) -> Result<Decoder<BufReader<File>>> {
    let path = PathBuf::from(&song.path);
    let file = std::fs::File::open(&song.path)?;
    let len = file.metadata()?.len();

    let mut builder = Decoder::builder()
        .with_data(BufReader::new(file))
        .with_byte_len(len)
        .with_seek_mode(SeekMode::Fastest)
        .with_seekable(true);

    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let hint = match ext {
            "adif" | "adts" => "aac",
            "caf" => "audio/x-caf",
            "m4a" | "m4b" | "m4p" | "m4r" | "mp4" => "audio/mp4",
            "bit" | "mpga" => "mp3",
            "mka" | "mkv" => "audio/matroska",
            "oga" | "ogm" | "ogv" | "ogx" | "spx" => "audio/ogg",
            "wave" => "wav",
            _ => ext,
        };
        builder = builder.with_hint(hint);
    }

    Ok(builder.build()?)
}

pub struct TappedSource<I> {
    input: I,
    metrics: Arc<PlaybackMetrics>,
}

impl<I> TappedSource<I> {
    pub fn new(input: I, metrics: Arc<PlaybackMetrics>) -> Self {
        TappedSource { input, metrics }
    }
}

impl<I> Iterator for TappedSource<I>
where
    I: Source<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(sample) = self.input.next() {
            if let Ok(mut samples) = self.metrics.audio_tap.try_lock() {
                if samples.len() >= super::OSCILLO_BUFFER_CAPACITY {
                    samples.pop_front();
                }
                samples.push_back(sample);
            }
            Some(sample)
        } else {
            None
        }
    }
}

impl<I> Source for TappedSource<I>
where
    I: Source<Item = f32>,
{
    fn channels(&self) -> ChannelCount {
        self.input.channels()
    }

    fn sample_rate(&self) -> NonZero<u32> {
        self.input.sample_rate()
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        self.input.total_duration()
    }

    fn current_span_len(&self) -> Option<usize> {
        self.input.current_span_len()
    }

    fn bits_per_sample(&self) -> Option<rodio::BitDepth> {
        self.input.bits_per_sample()
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), rodio::source::SeekError> {
        self.input.try_seek(pos)
    }
}
