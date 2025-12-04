use crate::{domain::SimpleSong, player::OSCILLO_BUFFER_CAPACITY};
use anyhow::Error;
use std::{collections::VecDeque, sync::Arc, time::Duration};

pub struct PlayerState {
    pub now_playing: Option<Arc<SimpleSong>>,
    pub state: PlaybackState,
    pub elapsed: Duration,
    pub oscilloscope_buffer: VecDeque<f32>,

    pub last_elapsed_secs: u64,
    pub elapsed_display: String,
    pub duration_display: String,

    pub player_error: Option<Error>,
}

impl Default for PlayerState {
    fn default() -> Self {
        PlayerState {
            state: PlaybackState::Stopped,
            now_playing: None,
            elapsed: Duration::default(),
            oscilloscope_buffer: VecDeque::with_capacity(OSCILLO_BUFFER_CAPACITY),

            last_elapsed_secs: 0,
            elapsed_display: String::with_capacity(11),
            duration_display: String::with_capacity(11),

            player_error: None,
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
