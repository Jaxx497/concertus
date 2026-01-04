use std::{
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    time::Duration,
};

use anyhow::Result;

use crate::player2::{
    backend_cplayback::ConcertusEngine, core::PlayerCore, metrics::PlaybackMetrics, ConcertusTrack,
    PlaybackState, PlayerCommand, PlayerEvent,
};

pub struct PlayerHandle {
    commands: Sender<PlayerCommand>,
    events: Receiver<PlayerEvent>,
    metrics: Arc<PlaybackMetrics>,
}

impl PlayerHandle {
    pub fn spawn() -> Self {
        let backend = ConcertusEngine::new().expect("Failed to initialize backend");
        // let backend = RodioBackend::new().expect("Failed to initialize backend");
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (evt_tx, evt_rx) = mpsc::channel();
        let metrics = PlaybackMetrics::new();

        PlayerCore::spawn(Box::new(backend), cmd_rx, evt_tx, Arc::clone(&metrics));

        Self {
            commands: cmd_tx,
            events: evt_rx,
            metrics,
        }
    }

    pub fn metrics(&self) -> Arc<PlaybackMetrics> {
        Arc::clone(&self.metrics)
    }
}

// =====================
//    COMMAND HANDLER
// =====================
impl PlayerHandle {
    pub fn play(&self, song: ConcertusTrack<u64>) -> Result<()> {
        self.commands.send(PlayerCommand::Play(song))?;
        Ok(())
    }

    pub fn set_next(&self, song: Option<ConcertusTrack<u64>>) -> Result<()> {
        self.commands.send(PlayerCommand::SetNext(song))?;
        Ok(())
    }

    pub fn clear_next(&self) -> Result<()> {
        self.commands.send(PlayerCommand::ClearNext)?;
        Ok(())
    }

    pub fn toggle_playback(&self) -> Result<()> {
        self.commands.send(PlayerCommand::TogglePlayback)?;
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        self.commands.send(PlayerCommand::Stop)?;
        Ok(())
    }

    pub fn seek_forward(&self, dur: u64) -> Result<()> {
        self.commands.send(PlayerCommand::SeekForward(dur))?;
        Ok(())
    }

    pub fn seek_back(&self, dur: u64) -> Result<()> {
        self.commands.send(PlayerCommand::SeekBack(dur))?;
        Ok(())
    }
}

// ===============
//    ACCESSORS
// ===============

impl PlayerHandle {
    pub fn elapsed(&self) -> Duration {
        self.metrics.get_elapsed()
    }

    pub fn get_playback_state(&self) -> PlaybackState {
        self.metrics.get_state()
    }

    pub fn is_paused(&self) -> bool {
        self.get_playback_state() == PlaybackState::Paused
    }

    pub fn is_stopped(&self) -> bool {
        self.get_playback_state() == PlaybackState::Stopped
    }

    pub fn poll_events(&mut self) -> Vec<PlayerEvent> {
        std::iter::from_fn(|| self.events.try_recv().ok()).collect()
    }
}
