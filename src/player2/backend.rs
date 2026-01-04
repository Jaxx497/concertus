use anyhow::Result;
use std::{path::Path, time::Duration};

pub(super) trait ConcertusBackend: Send + 'static {
    fn play(&mut self, path: &Path) -> Result<()>;
    fn stop(&mut self);
    fn pause(&mut self);
    fn resume(&mut self);

    fn seek_back(&mut self, secs: u64) -> Result<()>;
    fn seek_forward(&mut self, secs: u64) -> Result<()>;

    // State queries
    fn position(&self) -> Duration;
    fn is_paused(&self) -> bool;
    fn is_stopped(&self) -> bool;
    fn track_ended(&self) -> bool;

    // Optional features - default no-ops
    fn supports_gapless(&self) -> bool {
        false
    }
    fn set_next(&mut self, _path: &Path) -> Result<()> {
        Ok(()) // silently succeed if not supported
    }
    fn clear_queue(&mut self) {}

    fn drain_samples(&mut self) -> Vec<f32> {
        Vec::new()
    }
}
