use crate::domain::QueueSong;
use std::sync::Arc;

pub enum PlayerCommand {
    Play(Arc<QueueSong>),
    TogglePlayback,
    SeekForward(usize),
    SeekBack(usize),
    Stop,
}
