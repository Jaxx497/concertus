use crate::domain::SimpleSong;
use anyhow::Error;
use std::{sync::Arc, time::Duration};

pub struct PlayerState {
    pub now_playing: Option<Arc<SimpleSong>>,
    pub state: PlaybackState,
    pub elapsed: Duration,
    pub player_error: Option<Error>,
}

impl Default for PlayerState {
    fn default() -> Self {
        PlayerState {
            now_playing: None,
            elapsed: Duration::default(),
            state: PlaybackState::Stopped,
            player_error: None,
        }
    }
}

#[derive(PartialEq, Copy, Clone)]
pub enum PlaybackState {
    Playing,
    Paused,
    Stopped,
}
