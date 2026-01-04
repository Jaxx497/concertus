#![allow(unused)]
use anyhow::Result;
use rodio::decoder::builder::SeekMode;
use rodio::{ChannelCount, Decoder, OutputStream, OutputStreamBuilder, Sink, Source};
use std::{
    collections::VecDeque,
    fs::File,
    io::BufReader,
    num::NonZero,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use crate::player::ConcertusBackend;

pub struct RodioBackend {
    pub sink: Sink,
    track_ended: Arc<AtomicBool>,
    _stream: OutputStream,
    sample_buffer: Arc<Mutex<VecDeque<f32>>>,
}

impl RodioBackend {
    pub fn new() -> Result<Self> {
        let stream = OutputStreamBuilder::open_default_stream()?;
        let sink = Sink::connect_new(stream.mixer());

        Ok(Self {
            sink,
            _stream: stream,
            sample_buffer: Arc::new(Mutex::new(VecDeque::with_capacity(8192))),
            track_ended: Arc::new(AtomicBool::new(false)),
        })
    }
}

impl ConcertusBackend for RodioBackend {
    fn play(&mut self, song: &Path) -> Result<()> {
        let source = decode(song)?;

        self.track_ended.store(true, Ordering::SeqCst);
        let tapped = TappedSource::new(
            source,
            Arc::clone(&self.sample_buffer),
            Arc::clone(&self.track_ended),
        );

        self.sink.clear();
        self.sink.append(tapped);
        self.sink.play();

        Ok(())
    }

    fn pause(&mut self) {
        self.sink.pause();
    }

    fn resume(&mut self) {
        self.sink.play();
    }

    fn stop(&mut self) {
        self.sink.stop();
    }

    fn seek_forward(&mut self, secs: u64) -> Result<()> {
        let elapsed = self.position();
        let new_time = Duration::from_secs(secs) + elapsed;

        self.sink.try_seek(new_time)?;
        Ok(())
    }

    fn seek_back(&mut self, secs: u64) -> Result<()> {
        let elapsed = self.sink.get_pos();
        self.sink
            .try_seek(elapsed.saturating_sub(Duration::from_secs(secs)))?;
        Ok(())
    }

    fn position(&self) -> Duration {
        self.sink.get_pos()
    }

    fn is_paused(&self) -> bool {
        self.sink.is_paused()
    }

    fn is_stopped(&self) -> bool {
        self.sink.empty()
    }

    fn track_ended(&self) -> bool {
        self.track_ended.load(Ordering::SeqCst) && self.sink.empty()
    }

    fn drain_samples(&mut self) -> Vec<f32> {
        self.sample_buffer
            .lock()
            .map(|mut s| s.make_contiguous().to_vec())
            .unwrap_or_default()
    }
}

fn decode(song: &Path) -> Result<Decoder<BufReader<File>>> {
    let path = PathBuf::from(&song);
    let file = std::fs::File::open(&song)?;
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
    buffer: Arc<Mutex<VecDeque<f32>>>,
    ended: Arc<AtomicBool>,
}

impl<I> TappedSource<I> {
    pub fn new(input: I, buffer: Arc<Mutex<VecDeque<f32>>>, ended: Arc<AtomicBool>) -> Self {
        TappedSource {
            input,
            buffer,
            ended,
        }
    }
}

impl<I> Iterator for TappedSource<I>
where
    I: Source<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        match self.input.next() {
            Some(sample) => {
                if let Ok(mut samples) = self.buffer.try_lock() {
                    if samples.len() >= super::OSCILLO_BUFFER_CAPACITY {
                        samples.pop_front();
                    }
                    samples.push_back(sample);
                }
                Some(sample)
            }
            None => {
                self.ended.store(true, Ordering::SeqCst);
                None
            }
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
