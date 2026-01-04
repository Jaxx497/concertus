#![allow(unused)]

use anyhow::Result;
use ratatui::backend;
use rodio::{
    decoder::builder::SeekMode, ChannelCount, Decoder, OutputStream, OutputStreamBuilder, Sink,
    Source,
};
use std::{
    fs::File,
    io::BufReader,
    num::NonZero,
    ops::Sub,
    path::{Path, PathBuf},
    sync::{
        mpsc::{Receiver, Sender},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::{
    player::{self, OSCILLO_BUFFER_CAPACITY},
    player2::{
        track::ConcertusTrack, ConcertusBackend, PlaybackMetrics, PlaybackState, PlayerCommand,
        PlayerEvent,
    },
    REFRESH_RATE,
};

pub struct PlayerCore {
    backend: Box<dyn ConcertusBackend>,
    commands: Receiver<PlayerCommand>,
    events: Sender<PlayerEvent>,
    metrics: Arc<PlaybackMetrics>,

    current: Option<ConcertusTrack<u64>>,
    next: Option<ConcertusTrack<u64>>,
}

impl PlayerCore {
    pub fn spawn(
        backend: Box<dyn ConcertusBackend>,
        commands: Receiver<PlayerCommand>,
        events: Sender<PlayerEvent>,
        metrics: Arc<PlaybackMetrics>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            let mut core = PlayerCore {
                backend,
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
                PlayerCommand::Play(s) => self.play_song(s),
                PlayerCommand::SetNext(s) => self.set_next(s),
                PlayerCommand::ClearNext => self.clear_next(),
                PlayerCommand::TogglePlayback => self.toggle_playback(),
                PlayerCommand::Stop => self.stop(),
                PlayerCommand::SeekForward(x) => self.seek_forward(x),
                PlayerCommand::SeekBack(x) => self.seek_back(x),
            }
        }
    }

    fn check_track_end(&mut self) {
        // Checking status of `current` ensures the stop event is sent once
        if self.backend.track_ended() && self.current.is_some() {
            match self.next.take() {
                // GAPLESS BRANCH
                Some(next) => {
                    self.current = Some(next.clone());
                    let _ = self.events.send(PlayerEvent::TrackStarted(next));
                }
                // STANDARD BRANCH
                None => {
                    self.current = None;
                    let _ = self.events.send(PlayerEvent::PlaybackStopped);
                }
            }
        }
    }

    fn update_metrics(&mut self) {
        if self.current.is_some() {
            self.metrics.set_elapsed(self.backend.position())
        }
        self.tap_samples();
    }

    fn tap_samples(&mut self) {
        let samples = self.backend.drain_samples();

        if let Ok(mut tap) = self.metrics.audio_tap.try_lock() {
            for sample in samples {
                tap.push_back(sample);
                if tap.len() > OSCILLO_BUFFER_CAPACITY {
                    tap.pop_front();
                }
            }
        }
    }

    fn play_song(&mut self, song: ConcertusTrack<u64>) {
        self.backend.play(&song.get_path());
        self.current = Some(song.clone());

        self.metrics.set_playback_state(PlaybackState::Playing);

        let _ = self.events.send(PlayerEvent::TrackStarted(song));
    }

    fn set_next(&mut self, next: Option<ConcertusTrack<u64>>) {
        if self.backend.supports_gapless() {
            if let Some(song) = &next {
                self.backend.set_next(&song.get_path());
            }

            self.next = next;
        }
    }

    fn clear_next(&mut self) {
        self.next = None
    }

    fn toggle_playback(&mut self) {
        if self.backend.is_stopped() {
            return;
        }
        match self.backend.is_paused() {
            true => {
                self.backend.resume();
                self.metrics.set_playback_state(PlaybackState::Playing);
            }

            false => {
                self.backend.pause();
                self.metrics.set_playback_state(PlaybackState::Paused);
            }
        }
    }

    fn stop(&mut self) {
        self.backend.stop();
        self.current = None;
        self.metrics.reset();
        self.metrics.set_playback_state(PlaybackState::Stopped);
        self.backend.drain_samples();
    }

    fn seek_forward(&mut self, secs: u64) {
        if !self.backend.is_stopped() {
            let _ = self.backend.seek_forward(secs);
        }
    }

    fn seek_back(&mut self, secs: u64) {
        if !self.backend.is_stopped() {
            if let Err(e) = self.backend.seek_back(secs) {
                let _ = self.events.send(PlayerEvent::Error(e.to_string()));
            }
        }
    }
}
