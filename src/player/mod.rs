mod controller;
mod player;
mod state;
mod tapped_source;

pub use controller::PlayerController;
pub use player::Player;
pub use state::{PlaybackState, PlayerState};
pub use tapped_source::TappedSource;

use crate::domain::QueueSong;
use std::sync::Arc;

pub enum PlayerCommand {
    Play(Arc<QueueSong>),
    TogglePlayback,
    SeekForward(usize),
    SeekBack(usize),
    Stop,
}
