use crate::player2::{ConcertusBackend, OSCILLO_BUFFER_CAPACITY};
use anyhow::Result;
use std::{path::Path, time::Duration};

pub struct ConcertusEngine {
    engine: cplayback::Player,
}

impl ConcertusEngine {
    pub fn new() -> Result<Self> {
        Ok(Self {
            engine: cplayback::Player::new()?,
        })
    }
}

impl ConcertusBackend for ConcertusEngine {
    fn play(&mut self, song: &Path) -> Result<()> {
        self.engine.clear();
        self.engine.play_blocking(song)?;

        Ok(())
    }

    fn pause(&mut self) {
        self.engine.pause();
    }

    fn resume(&mut self) {
        self.engine.pause();
    }

    fn stop(&mut self) {
        let _ = self.engine.stop();
    }

    fn seek_forward(&mut self, secs: u64) -> Result<()> {
        let elapsed = self.engine.position();

        let new_time = Duration::from_secs(secs) + elapsed;
        self.engine.seek(new_time)?;
        Ok(())
    }

    fn seek_back(&mut self, secs: u64) -> Result<()> {
        let elapsed = self.engine.position();
        let new_time = elapsed.saturating_sub(Duration::from_secs(secs));
        self.engine.seek(new_time)?;
        Ok(())
    }

    fn position(&self) -> Duration {
        self.engine.position()
    }

    fn is_paused(&self) -> bool {
        self.engine.is_paused()
    }

    fn is_stopped(&self) -> bool {
        self.engine.is_stopped()
    }

    fn track_ended(&self) -> bool {
        self.engine.track_ended()
    }

    fn supports_gapless(&self) -> bool {
        true
    }

    fn set_next(&mut self, song: &Path) -> Result<()> {
        self.engine.queue(&song)?;
        Ok(())
    }

    fn drain_samples(&mut self) -> Vec<f32> {
        self.engine.tap.latest(OSCILLO_BUFFER_CAPACITY)
    }
}
