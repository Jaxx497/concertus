use crate::domain::SimpleSong;
use std::sync::Arc;

pub enum PlayerEvent {
    TrackStarted(Arc<SimpleSong>),
    EndOfStream(Arc<SimpleSong>),
    PlaybackStopped,
    Error(String),
}
