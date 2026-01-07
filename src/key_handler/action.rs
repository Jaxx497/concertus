use crate::{
    key_handler::*,
    ui_state::{
        LibraryView, Mode, Pane, PlaylistAction, PopupType, ProgressDisplay, SettingsMode, UiState,
    },
    REFRESH_RATE,
};
use anyhow::Result;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent};

use KeyCode::*;

#[rustfmt::skip]
pub fn handle_key_event(key_event: KeyEvent, state: &UiState) -> Option<Action> {
    if let Some(action) = global_commands(&key_event, &state) {
        return Some(action);
    }

    match state.get_input_context() {
        InputContext::Popup(popup)  => handle_popup(&key_event, &popup),
        InputContext::Fullscreen    => handle_fullscreen(&key_event),
        InputContext::TrackList(_)  => handle_tracklist(&key_event, &state),
        InputContext::AlbumView     => handle_album_browser(&key_event),
        InputContext::PlaylistView  => handle_playlist_browswer(&key_event),
        InputContext::Search        => handle_search_pane(&key_event, &state),

        _ => None,
    }
}

fn global_commands(key: &KeyEvent, state: &UiState) -> Option<Action> {
    let in_search = state.get_pane() == Pane::Search;
    let fullscreen = matches!(state.get_mode(), Mode::Fullscreen);
    let popup_active = state.popup.is_open();

    // Works on every pane, even search
    match (key.modifiers, key.code) {
        (C, Char('c')) => Some(Action::QUIT),

        (C, Char(' ')) => Some(Action::TogglePlayback),

        (C, Char('n')) => Some(Action::PlayNext),
        (C, Char('p')) => Some(Action::PlayPrev),

        (X, Media(event::MediaKeyCode::PlayPause)) => Some(Action::TogglePlayback),

        // Works on everything except search or popup
        _ if (!in_search && !popup_active && !fullscreen) => match (key.modifiers, key.code) {
            // PLAYBACK COMMANDS
            (X, Esc) => Some(Action::SoftReset),

            (S, Char('C')) => Some(Action::ThemeManager),
            (X, F(6)) => Some(Action::ThemeRefresh),

            (C, Char('t')) => Some(Action::ChangeMode(Mode::Library(LibraryView::Playlists))),
            (C, Char('q')) => Some(Action::ChangeMode(Mode::Queue)),
            (C, Char('z')) => Some(Action::ChangeMode(Mode::Power)),

            (X, Char('`')) => Some(Action::ViewSettings),
            (X, Char(' ')) => Some(Action::TogglePlayback),
            (C, Char('s')) => Some(Action::Stop),

            (X, Char('n')) => Some(Action::SeekForward(SEEK_SMALL)),
            (S, Char('N')) => Some(Action::SeekForward(SEEK_LARGE)),

            (X, Char('p')) => Some(Action::SeekBack(SEEK_SMALL)),
            (S, Char('P')) => Some(Action::SeekBack(SEEK_LARGE)),

            // NAVIGATION
            (X, Char('/')) => Some(Action::ChangeMode(Mode::Search)),

            (X, Char('1')) => Some(Action::ChangeMode(Mode::Library(LibraryView::Albums))),
            (X, Char('2')) => Some(Action::ChangeMode(Mode::Library(LibraryView::Playlists))),
            (X, Char('3')) => Some(Action::ChangeMode(Mode::Queue)),
            (X, Char('0')) => Some(Action::ChangeMode(Mode::Power)),

            // SCROLLING
            (X, Char('j')) | (X, Down) => Some(Action::Scroll(Director::Down(1))),
            (X, Char('k')) | (X, Up) => Some(Action::Scroll(Director::Up(1))),
            (X, Char('d')) => Some(Action::Scroll(Director::Down(SCROLL_MID))),
            (X, Char('u')) => Some(Action::Scroll(Director::Up(SCROLL_MID))),
            (S, Char('D')) => Some(Action::Scroll(Director::Down(SCROLL_XTRA))),
            (S, Char('U')) => Some(Action::Scroll(Director::Up(SCROLL_XTRA))),
            (X, Char('g')) => Some(Action::Scroll(Director::Top)),
            (S, Char('G')) => Some(Action::Scroll(Director::Bottom)),

            (X, Char('[')) => Some(Action::IncrementSidebarSize(-SIDEBAR_INCREMENT)),
            (X, Char(']')) => Some(Action::IncrementSidebarSize(SIDEBAR_INCREMENT)),

            (S, Char('{')) => Some(Action::IncrementWFSmoothness(Incrementor::Down)),
            (S, Char('}')) => Some(Action::IncrementWFSmoothness(Incrementor::Up)),

            (S, Char('<')) => Some(Action::CycleTheme(Incrementor::Up)),
            (S, Char('>')) => Some(Action::CycleTheme(Incrementor::Down)),

            (_, Char('f') | Char('F')) => Some(Action::ChangeMode(Mode::Fullscreen)),
            (X, Char('w')) => Some(Action::SetProgressDisplay(ProgressDisplay::Waveform)),
            (X, Char('o')) => Some(Action::SetProgressDisplay(ProgressDisplay::Oscilloscope)),
            (X, Char('b')) => Some(Action::SetProgressDisplay(ProgressDisplay::ProgressBar)),
            (S, Char('W')) => Some(Action::SetFullscreen(ProgressDisplay::Waveform)),
            (S, Char('O')) => Some(Action::SetFullscreen(ProgressDisplay::Oscilloscope)),
            (S, Char('B')) => Some(Action::SetFullscreen(ProgressDisplay::ProgressBar)),
            (C, Char('u')) | (X, F(5)) => Some(Action::UpdateLibrary),

            _ => None,
        },
        _ => None,
    }
}

fn handle_tracklist(key: &KeyEvent, state: &UiState) -> Option<Action> {
    let base_action = match (key.modifiers, key.code) {
        (X, Enter) => Some(Action::Play),

        (X, Char('a')) => Some(Action::AddToPlaylist),
        (C, Char('a')) => Some(Action::GoToAlbum),
        (X, Char('q')) => Some(Action::QueueSong),
        (X, Char('v')) => Some(Action::MultiSelect),
        (C, Char('v')) => Some(Action::ClearMultiSelect),

        (X, Left) | (X, Char('h') | Tab) => Some(Action::ChangeMode(Mode::Library(
            state.display_state.sidebar_view,
        ))),
        _ => None,
    };

    if base_action.is_some() {
        return base_action;
    }

    match state.get_mode() {
        Mode::Library(_) => match (key.modifiers, key.code) {
            (S, Char('K')) => Some(Action::ShiftPosition(Incrementor::Up)),
            (S, Char('J')) => Some(Action::ShiftPosition(Incrementor::Down)),
            (S, Char('Q')) => Some(Action::QueueMany {
                sel_type: SelectionType::Multi,
                shuffle: false,
            }),
            (S, Char('V')) => Some(Action::MultiSelectAll),
            (X, Char('s')) => Some(Action::QueueMany {
                sel_type: SelectionType::Multi,
                shuffle: true,
            }),
            (X, Char('x')) => Some(Action::RemoveSong),
            _ => None,
        },

        Mode::Queue => match (key.modifiers, key.code) {
            (X, Char('x')) => Some(Action::RemoveSong),
            (X, Char('s')) => Some(Action::ShuffleElements),
            (S, Char('V')) => Some(Action::MultiSelectAll),

            (S, Char('K')) => Some(Action::ShiftPosition(Incrementor::Up)),
            (S, Char('J')) => Some(Action::ShiftPosition(Incrementor::Down)),
            _ => None,
        },

        Mode::Power | Mode::Search => match (key.modifiers, key.code) {
            (C, Left) | (C, Char('h')) => Some(Action::SortColumnsPrev),
            (C, Right) | (C, Char('l')) => Some(Action::SortColumnsNext),
            _ => None,
        },
        _ => None,
    }
}

fn handle_album_browser(key: &KeyEvent) -> Option<Action> {
    match (key.modifiers, key.code) {
        (X, Char('q')) => Some(Action::QueueMany {
            sel_type: SelectionType::Album,
            shuffle: false,
        }),
        (X, Enter) | (X, Tab) | (X, Right) | (X, Char('l')) | (C, Char('a')) => {
            Some(Action::ChangePane(Pane::TrackList))
        }
        (X, Char('s')) => Some(Action::QueueMany {
            sel_type: SelectionType::Album,
            shuffle: true,
        }),

        // Change album sorting algorithm
        (C, Left) | (C, Char('h')) => Some(Action::ToggleAlbumSort(false)),
        (C, Right) | (C, Char('l')) => Some(Action::ToggleAlbumSort(true)),

        _ => None,
    }
}

fn handle_playlist_browswer(key: &KeyEvent) -> Option<Action> {
    match (key.modifiers, key.code) {
        (C, Char('a')) => Some(Action::ChangeMode(Mode::Library(LibraryView::Albums))),
        (X, Char('r')) => Some(Action::RenamePlaylist),
        (X, Char('q')) => Some(Action::QueueMany {
            sel_type: SelectionType::Playlist,
            shuffle: false,
        }),

        (X, Enter) | (X, Tab) | (X, Right) | (X, Char('l')) => {
            Some(Action::ChangePane(Pane::TrackList))
        }

        (X, Char('c')) => Some(Action::CreatePlaylist),
        (C, Char('d')) => Some(Action::DeletePlaylist),
        (X, Char('s')) => Some(Action::QueueMany {
            sel_type: SelectionType::Playlist,
            shuffle: true,
        }),
        _ => None,
    }
}

fn handle_search_pane(key: &KeyEvent, state: &UiState) -> Option<Action> {
    match (key.modifiers, key.code) {
        (X, Esc) => Some(Action::ChangeMode(Mode::Library(
            state.display_state.sidebar_view,
        ))),
        (X, Tab) | (X, Enter) => Some(Action::SendSearch),
        (C, Char('a')) => Some(Action::ChangeMode(Mode::Library(LibraryView::Albums))),

        (_, Left) | (C, Char('h')) => Some(Action::SortColumnsPrev),
        (_, Right) | (C, Char('l')) => Some(Action::SortColumnsNext),
        (C, Enter) | (S, Enter) => None,
        (_, Char(x)) if ILLEGAL_CHARS.contains(&x) => None,

        _ => Some(Action::UpdateSearch(*key)),
    }
}

fn handle_fullscreen(key: &KeyEvent) -> Option<Action> {
    let action = match (key.modifiers, key.code) {
        (X, Char(' ')) => Action::TogglePlayback,

        (X, Char('n')) => Action::SeekForward(SEEK_SMALL),
        (S, Char('N')) => Action::SeekForward(SEEK_LARGE),

        (X, Char('p')) => Action::SeekBack(SEEK_SMALL),
        (S, Char('P')) => Action::SeekBack(SEEK_LARGE),

        (X, Char('w')) | (S, Char('W')) => Action::SetProgressDisplay(ProgressDisplay::Waveform),
        (X, Char('o')) | (S, Char('O')) => {
            Action::SetProgressDisplay(ProgressDisplay::Oscilloscope)
        }
        (X, Char('b')) | (S, Char('B')) => Action::SetProgressDisplay(ProgressDisplay::ProgressBar),

        (S, Char('{')) => Action::IncrementWFSmoothness(Incrementor::Down),
        (S, Char('}')) => Action::IncrementWFSmoothness(Incrementor::Up),

        _ => Action::RevertFullscreen,
    };

    Some(action)
}

fn handle_popup(key: &KeyEvent, popup: &PopupType) -> Option<Action> {
    match popup {
        PopupType::Settings(s) => root_manager(key, s),
        PopupType::Playlist(p) => handle_playlist(key, p),
        PopupType::ThemeManager => handle_themeing(key),
        PopupType::Error(_) => Some(Action::ClosePopup),
        _ => None,
    }
}

fn root_manager(key: &KeyEvent, variant: &SettingsMode) -> Option<Action> {
    use SettingsMode::*;
    match variant {
        ViewRoots => match key.code {
            Char('a') => Some(Action::RootAdd),
            Char('d') => Some(Action::RootRemove),
            Up | Char('k') => Some(Action::PopupScrollUp),
            Down | Char('j') => Some(Action::PopupScrollDown),
            Char('`') => Some(Action::ClosePopup),
            Esc => Some(Action::ClosePopup),
            _ => None,
        },
        AddRoot => match key.code {
            Esc => Some(Action::ViewSettings),
            Enter => Some(Action::RootConfirm),
            _ => Some(Action::PopupInput(*key)),
        },
        RemoveRoot => match key.code {
            Esc => Some(Action::ViewSettings),
            Enter => Some(Action::RootConfirm),
            _ => None,
        },
    }
}

fn handle_playlist(key: &KeyEvent, variant: &PlaylistAction) -> Option<Action> {
    use PlaylistAction::*;

    if key.code == Esc {
        return Some(Action::ClosePopup);
    }

    match variant {
        Create => match key.code {
            Enter => Some(Action::CreatePlaylistConfirm),
            _ => Some(Action::PopupInput(*key)),
        },
        Delete => match key.code {
            Enter => Some(Action::DeletePlaylistConfirm),
            _ => Some(Action::PopupInput(*key)),
        },
        AddSong => match key.code {
            Up | Char('k') => Some(Action::PopupScrollUp),
            Down | Char('j') => Some(Action::PopupScrollDown),
            Enter | Char('a') => Some(Action::AddToPlaylistConfirm),
            Char('c') => Some(Action::CreatePlaylistWithSongs),
            _ => None,
        },
        CreateWithSongs => match key.code {
            Enter => Some(Action::CreatePlaylistWithSongsConfirm),
            _ => Some(Action::PopupInput(*key)),
        },
        Rename => match key.code {
            Enter => Some(Action::RenamePlaylistConfirm),
            _ => Some(Action::PopupInput(*key)),
        },
    }
}

fn handle_themeing(key: &KeyEvent) -> Option<Action> {
    match key.code {
        Up | Char('k') => Some(Action::PopupScrollUp),
        Down | Char('j') => Some(Action::PopupScrollDown),
        _ => Some(Action::ClosePopup),
    }
}

pub fn next_event() -> Result<Option<Event>> {
    match event::poll(REFRESH_RATE)? {
        true => Ok(Some(event::read()?)),
        false => Ok(None),
    }
}
