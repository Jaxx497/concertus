use crate::domain::SimpleSong;
use anyhow::Error;
use std::{collections::VecDeque, sync::Arc, time::Duration};

pub struct PlayerState {
    pub now_playing: Option<Arc<SimpleSong>>,
    pub state: PlaybackState,
    pub elapsed: Duration,
    pub player_error: Option<Error>,
    pub oscilloscope_buffer: VecDeque<f32>,
    pub elapsed_display: String,
    pub duration_display: String,
    pub last_elapsed_secs: u64,
}

impl Default for PlayerState {
    fn default() -> Self {
        PlayerState {
            now_playing: None,
            elapsed: Duration::default(),
            state: PlaybackState::Stopped,
            player_error: None,
            oscilloscope_buffer: VecDeque::with_capacity(1024),
            duration_display: String::new(),
            elapsed_display: String::new(),

            last_elapsed_secs: 0,
        }
    }
}

#[derive(PartialEq, Copy, Clone)]
pub enum PlaybackState {
    Playing,
    Paused,
    Transitioning,
    Stopped,
}
