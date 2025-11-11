mod action;

use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::LazyLock;
use std::time::Duration;
use std::time::Instant;

pub use action::handle_key_event;
pub use action::next_event;
use ratatui::crossterm::event::KeyCode;
use ratatui::crossterm::event::KeyEvent;
use ratatui::crossterm::event::KeyModifiers;

use crate::ui_state::Mode;
use crate::ui_state::Pane;
use crate::ui_state::PopupType;
use crate::ui_state::ProgressDisplay;

static ILLEGAL_CHARS: LazyLock<HashSet<char>> = LazyLock::new(|| HashSet::from([';']));

const X: KeyModifiers = KeyModifiers::NONE;
const S: KeyModifiers = KeyModifiers::SHIFT;
const C: KeyModifiers = KeyModifiers::CONTROL;

const SEEK_SMALL: usize = 5;
const SEEK_LARGE: usize = 30;
const SCROLL_MID: usize = 5;
const SCROLL_XTRA: usize = 20;
const SIDEBAR_INCREMENT: isize = 1;

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
    ShuffleEntity,
    RemoveSong,

    AddToPlaylist,
    AddToPlaylistConfirm,

    CreatePlaylistWithSongs,
    CreatePlaylistWithSongsConfirm,

    // Updating App State
    UpdateLibrary,
    SendSearch,
    UpdateSearch(KeyEvent),
    SortColumnsNext,
    SortColumnsPrev,
    ToggleAlbumSort(bool),
    ChangeMode(Mode),
    ChangePane(Pane),
    GoToAlbum,
    Scroll(Director),

    MultiSelect,
    MultiSelectAll,
    ClearMultiSelect,

    // Playlists
    CreatePlaylist,
    CreatePlaylistConfirm,

    DeletePlaylist,
    DeletePlaylistConfirm,

    RenamePlaylist,
    RenamePlaylistConfirm,

    ShiftPosition(MoveDirection),
    ShuffleElements,

    // Display
    CycleTheme(MoveDirection),
    ThemeManager,
    ThemeRefresh,

    IncrementWFSmoothness(MoveDirection),
    IncrementSidebarSize(isize),

    SetProgressDisplay(ProgressDisplay),
    ToggleProgressDisplay,
    SetFullscreen(ProgressDisplay),
    RevertFullscreen,

    PopupScrollUp,
    PopupScrollDown,
    PopupInput(KeyEvent),

    ClosePopup,

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
    Fullscreen,
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

thread_local! {
    static LAST_KEY_TIME: RefCell<Option<Instant>> = RefCell::new(None);
}

const PASTE_THRESHOLD: Duration = Duration::from_millis(10);

pub fn is_likely_paste() -> bool {
    LAST_KEY_TIME.with(|last_time| {
        let mut last = last_time.borrow_mut();

        let is_paste = match *last {
            Some(prev) => prev.elapsed() < PASTE_THRESHOLD,
            None => false,
        };
        *last = Some(Instant::now());
        is_paste
    })
}

pub struct ScrollAccelerator {
    key_states: HashMap<KeyCode, (Instant, usize)>,
}

impl ScrollAccelerator {
    pub fn new() -> Self {
        ScrollAccelerator {
            key_states: HashMap::new(),
        }
    }

    pub fn get_scroll_multiplier(&mut self, key: KeyCode) -> usize {
        let now = Instant::now();

        let (first, count) = self.key_states.entry(key).or_insert((now, 0));

        *count += 1;

        let held_duration = now.duration_since(*first);

        match held_duration.as_millis() {
            0..=300 => 1,
            _ => 2,
        }
    }

    pub fn reset(&mut self, key: KeyCode) {
        self.key_states.remove(&key);
    }

    pub fn reset_all(&mut self) {
        self.key_states.clear();
    }
}

thread_local! {static SCROLL_ACCEL: RefCell<ScrollAccelerator> = RefCell::new(ScrollAccelerator::new())}

pub fn on_key_release(key_code: KeyCode) {
    SCROLL_ACCEL.with(|accel| {
        accel.borrow_mut().reset(key_code);
    });
}
