use super::{PlaybackState, PlayerState};
use crate::{
    domain::{QueueSong, SongInfo},
    get_readable_duration,
};
use anyhow::Result;
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, decoder::builder::SeekMode};
use std::{
    fs::File,
    io::BufReader,
    ops::Sub,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

pub struct Player {
    sink: Sink,
    pub shared_state: Arc<Mutex<PlayerState>>,
    _stream: OutputStream,
}

impl Player {
    pub(crate) fn new(shared_state: Arc<Mutex<PlayerState>>) -> Self {
        let _stream = OutputStreamBuilder::open_default_stream().expect("Cannot open stream");
        let sink = Sink::connect_new(_stream.mixer());

        Player {
            sink,
            shared_state,
            _stream,
        }
    }

    /// Play a song
    /// Returns an error if
    pub(crate) fn play_song(&mut self, song: &Arc<QueueSong>) -> Result<()> {
        let source = decode(song)?;

        self.sink.clear();
        self.sink.append(source);
        self.sink.play();

        let mut player_state = self
            .shared_state
            .lock()
            .expect("Failed to unwrap mutex in music player");
        player_state.state = PlaybackState::Playing;
        player_state.now_playing = Some(Arc::clone(&song.meta));
        player_state.elapsed = Duration::default();
        player_state.duration_display =
            get_readable_duration(song.meta.duration, crate::DurationStyle::Compact);
        player_state.elapsed_display = "0:00".to_string();

        Ok(())
    }

    /// Toggles the playback state of the audio player.
    ///
    /// This function manages the playback state transitions:
    /// - If no track is currently loaded (`now_playing` is None), it sets the state to `Stopped`.
    /// - If a track is loaded and currently paused, it resumes playback.
    /// - If a track is loaded and currently playing or in any other state, it pauses playback.
    ///
    /// # State Transitions
    /// - `None` -> `Stopped`
    /// - `Paused` -> `Playing` (resumes playback)
    /// - `Playing` or any other state -> `Paused`
    ///
    /// # Effects
    /// - When resuming, it calls `play()` on the sink.
    /// - When pausing, it calls `pause()` on the sink.
    ///
    /// # Examples
    /// ```
    /// let mut player = AudioPlayer::new();
    /// player.toggle_playback();       // Does nothing
    /// player.play_song(some_track);   // Starts playing
    /// player.toggle_playback();       // Pauses
    /// player.toggle_playback();       // Resumes playing
    /// ```
    pub(crate) fn toggle_playback(&mut self) {
        let (now_playing, playback_state) = {
            let state = self
                .shared_state
                .lock()
                .expect("Failed to unwrap mutex in music player");
            (state.now_playing.is_none(), state.state)
        };

        let mut state = self
            .shared_state
            .lock()
            .expect("Failed to unwrap mutex in music player");
        match (now_playing, playback_state) {
            (true, _) => state.state = PlaybackState::Stopped,

            //  RESUMING PLAYBACK
            (false, PlaybackState::Paused) => {
                self.sink.play();
                state.state = PlaybackState::Playing;
            }

            // PAUSING THE SINK
            (false, _) => {
                self.sink.pause();
                state.state = PlaybackState::Paused;
            }
        }
    }

    // /// Stop playback
    pub(crate) fn stop(&mut self) {
        self.sink.clear();

        let mut state = self
            .shared_state
            .lock()
            .expect("Failed to unwrap mutex in music player");
        state.now_playing = None;
        state.elapsed = Duration::default();
        state.state = PlaybackState::Stopped;
    }

    // BUG: Due to the development status of the symphonia crate, some decoders do not
    // implement seeking. FLAC files are dodgy, and often fail while testing in DEBUG
    // mode, however most problems seem to be solved in RELEASE mode. OGG files fail
    // with a 100% rate regardless of mode.
    // --
    // We'll try testing the symphonia 0.6 branch at some point to see how it fares.

    /// Fast forwards playback 5 seconds
    /// Will skip to next track if in last 5 seconds
    pub(crate) fn seek_forward(&mut self, secs: usize) -> Result<()> {
        let (now_playing, playback_state) = {
            let state = self
                .shared_state
                .lock()
                .expect("Failed to unwrap mutex in music player");
            (state.now_playing.clone(), state.state)
        };

        if playback_state != PlaybackState::Stopped {
            let elapsed = self.sink.get_pos();
            let duration = &now_playing.unwrap().duration;

            let mut state = self
                .shared_state
                .lock()
                .expect("Failed to unwrap mutex in music player");

            // This prevents skiping into the next song's playback
            if duration.sub(elapsed) > Duration::from_secs_f32(secs as f32 + 0.5) {
                let new_time = elapsed + Duration::from_secs(secs as u64);
                if let Err(_) = self.sink.try_seek(new_time) {
                    self.sink.clear();
                    state.state = PlaybackState::Stopped;
                } else {
                    state.elapsed = self.sink.get_pos()
                }
            } else {
                self.sink.clear();
                state.state = PlaybackState::Stopped;
            }
        }
        Ok(())
    }

    /// Rewinds playback 5 seconds
    pub(crate) fn seek_back(&mut self, secs: usize) {
        let playback_state = {
            let state = self
                .shared_state
                .lock()
                .expect("Failed to unwrap mutex in music player");
            state.state
        };

        if playback_state != PlaybackState::Stopped {
            let elapsed = self.sink.get_pos();

            match elapsed < Duration::from_secs(secs as u64) {
                true => {
                    let _ = self.sink.try_seek(Duration::from_secs(0));
                }
                false => {
                    let new_time = elapsed.sub(Duration::from_secs(secs as u64));
                    let _ = self.sink.try_seek(new_time);
                }
            }

            let mut state = self
                .shared_state
                .lock()
                .expect("Failed to unwrap mutex in music player");
            state.elapsed = self.sink.get_pos()
        }
    }

    pub(crate) fn update_elapsed(&self) {
        if let Ok(mut state) = self.shared_state.lock() {
            if state.state == PlaybackState::Playing {
                let new_elapsed = self.sink.get_pos();
                state.elapsed = new_elapsed;

                let secs = new_elapsed.as_secs();
                if secs != state.last_elapsed_secs {
                    state.last_elapsed_secs = secs;
                    state.elapsed_display =
                        get_readable_duration(new_elapsed, crate::DurationStyle::Compact);
                }
            }
        }
    }

    pub(crate) fn sink_is_empty(&self) -> bool {
        self.sink.empty()
    }
}

fn decode(song: &Arc<QueueSong>) -> Result<Decoder<BufReader<File>>> {
    let path = PathBuf::from(&song.path);
    let file = std::fs::File::open(&song.path)?;
    let duration = song.get_duration();

    let mut builder = Decoder::builder()
        .with_data(BufReader::new(file))
        .with_total_duration(duration)
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
