mod action;

use std::collections::HashSet;
use std::sync::LazyLock;

pub use action::handle_key_event;
pub use action::next_event;
use ratatui::crossterm::event::KeyEvent;
use ratatui::crossterm::event::KeyModifiers;

use crate::ui_state::Mode;
use crate::ui_state::Pane;
use crate::ui_state::PopupType;

static ILLEGAL_CHARS: LazyLock<HashSet<char>> = LazyLock::new(|| HashSet::from([';']));

const X: KeyModifiers = KeyModifiers::NONE;
const S: KeyModifiers = KeyModifiers::SHIFT;
const C: KeyModifiers = KeyModifiers::CONTROL;

const SEEK_SMALL: usize = 5;
const SEEK_LARGE: usize = 30;
const SCROLL_MID: usize = 5;
const SCROLL_XTRA: usize = 20;

#[derive(PartialEq, Eq)]
pub enum Action {
    // Player Controls
    Play,
    Stop,
    TogglePause,
    PlayNext,
    PlayPrev,
    SeekForward(usize),
    SeekBack(usize),

    // Queue & Playlist Actions
    QueueSong,
    QueueEntity,
    RemoveSong,

    AddToPlaylist,
    AddToPlaylistConfirm,

    // Updating App State
    UpdateLibrary,
    SendSearch,
    UpdateSearch(KeyEvent),
    SortColumnsNext,
    SortColumnsPrev,
    ToggleAlbumSort(bool),
    ToggleSideBar,
    ChangeMode(Mode),
    ChangePane(Pane),
    GoToAlbum,
    Scroll(Director),

    BulkSelect,
    BulkSelectALL,
    ClearBulkSelect,

    // Playlists
    CreatePlaylist,
    CreatePlaylistConfirm,

    DeletePlaylist,
    DeletePlaylistConfirm,

    ShiftPosition(MoveDirection),

    ClosePopup,
    PopupScrollUp,
    PopupScrollDown,
    PopupInput(KeyEvent),

    // Errors, Convenience & Other
    ViewSettings,
    RootAdd,
    RootRemove,
    RootConfirm,

    HandleErrors,
    SoftReset,
    QUIT,
}

pub enum InputContext {
    AlbumView,
    PlaylistView,
    TrackList(Mode),
    Search,
    Queue,
    Popup(PopupType),
}

#[derive(PartialEq, Eq)]
pub enum Director {
    Up(usize),
    Down(usize),
    Top,
    Bottom,
}

#[derive(PartialEq, Eq)]
pub enum MoveDirection {
    Up,
    Down,
}
