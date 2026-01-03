#![allow(unused)]

use crate::{
    domain::{QueueSong, SimpleSong},
    player2::{PlaybackMetrics, PlayerCommand, PlayerEvent},
    REFRESH_RATE,
};
use std::{
    ops::Sub,
    sync::{
        mpsc::{Receiver, Sender},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

pub struct PlayerCore {
    backend: cplayback::Player,
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
            let engine = cplayback::Player::new().expect("Could not initialize CPLAYBACK engine");

            let mut core = PlayerCore {
                backend: engine,
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
        if self.backend.track_ended() && self.current.is_some() {
            self.current = None;
            let _ = self.events.send(PlayerEvent::PlaybackStopped);
        }
    }

    fn update_metrics(&mut self) {
        if self.current.is_some() {
            self.metrics.set_elapsed(self.backend.position())
        }
    }
}

impl PlayerCore {
    fn play_song(&mut self, song: &Arc<QueueSong>) {
        let player = &self.backend;

        player.clear();
        if let Err(e) = player.play_blocking(&song.path) {
            let _ = self.events.send(PlayerEvent::Error(e.to_string()));
        };

        self.current = Some(Arc::clone(&song.meta));

        let _ = self
            .events
            .send(PlayerEvent::TrackStarted(Arc::clone(&song)));
    }

    fn set_next(&mut self, song: Option<Arc<QueueSong>>) {
        if let Some(s) = &song {
            let _ = self.backend.queue(&s.path);
        }
        self.next = song;
    }

    fn toggle_playback(&mut self) {
        if self.backend.is_stopped() {
            return;
        }
        match self.backend.is_playing() {
            true => self.backend.pause(),
            false => self.backend.resume(),
        }
    }

    pub(crate) fn stop(&mut self) {
        let _ = self.backend.stop();
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
                if let Err(e) = self.backend.seek(new_time) {
                    let _ = self.events.send(PlayerEvent::Error(e.to_string()));
                };
            } else {
                if let Err(e) = self.backend.seek(duration.sub(Duration::from_millis(10))) {
                    let _ = self.events.send(PlayerEvent::Error(e.to_string()));
                };
            }
        }
    }

    pub(crate) fn seek_back(&mut self, secs: u64) {
        if let Some(_) = &self.current {
            let elapsed = self.metrics.get_elapsed();

            if elapsed.as_secs_f32() > 5.0 {
                let _ = self.backend.seek(elapsed.sub(Duration::from_secs(secs)));
            } else {
                let _ = self.backend.seek(Duration::ZERO);
            }
        }
    }
}
