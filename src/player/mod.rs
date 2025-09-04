mod controller;
mod player;
mod state;

pub use controller::PlayerController;
pub use player::Player;
pub use state::{PlaybackState, PlayerState};

use crate::domain::QueueSong;
use std::sync::Arc;

pub enum PlayerCommand {
    Play(Arc<QueueSong>),
    TogglePlayback,
    SeekForward(usize),
    SeekBack(usize),
    Stop,
}
