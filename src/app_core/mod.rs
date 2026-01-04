use crate::{player::PlayerHandle, ui_state::UiState, Library};
use crossbeam_channel::Receiver;
use std::sync::Arc;

mod app;
mod key_events;
mod library;
mod playback;
mod player;
mod select;

pub use key_events::key_loop;

pub struct Concertus {
    library: Arc<Library>,
    pub(crate) ui: UiState,
    player: PlayerHandle,
    library_refresh_rec: Option<Receiver<LibraryRefreshProgress>>,
}

pub enum LibraryRefreshProgress {
    Scanning {
        progress: u8,
    },
    Processing {
        progress: u8,
        current: usize,
        total: usize,
    },
    UpdatingDatabase {
        progress: u8,
    },
    Rebuilding {
        progress: u8,
    },
    Complete(crate::Library),
    Error(String),
}
