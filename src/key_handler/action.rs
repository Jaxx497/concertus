use crate::{
    app_core::Concertus,
    ui_state::{LibraryView, Mode, Pane, PlaylistAction, PopupType, SettingsMode, UiState},
    REFRESH_RATE,
};
use anyhow::{anyhow, Result};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::{collections::HashSet, sync::LazyLock, time::Duration};

static ILLEGAL_CHARS: LazyLock<HashSet<char>> = LazyLock::new(|| HashSet::from(['\'', ';']));
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

    // Playlists
    CreatePlaylist,
    CreatePlaylistConfirm,

    DeletePlaylist,
    DeletePlaylistConfirm,


    PopupInput(KeyEvent),

    ClosePopup,
    PopupScrollUp,
    PopupScrollDown,

    // Errors, Convenience & Other
    ViewSettings,
    RootAdd,
    RootRemove,
    RootConfirm,

    // SettingsCancel,
    SettingsInput(KeyEvent),

    HandleErrors,
    SoftReset,
    QUIT,
}

#[derive(PartialEq, Eq)]
pub enum Director {
    Up(usize),
    Down(usize),
    Top,
    Bottom,
}

use KeyCode::*;

#[rustfmt::skip]
pub fn handle_key_event(key_event: KeyEvent, state: &UiState) -> Option<Action> {
   
    let pane = state.get_pane();

    if let Some(action) = global_commands(&key_event, &state) {
        return Some(action)
    } 

    if state.popup.is_open() {
        return handle_popup(&key_event, state)
    };

    match pane {
        Pane::TrackList => handle_main_pane(&key_event, &state),
        Pane::Search    => handle_search_pane(&key_event),
        Pane::SideBar   => handle_sidebar_pane(&key_event),
        _ => None

    }
}

fn global_commands(key: &KeyEvent, state: &UiState) -> Option<Action> {
    let in_search = state.get_pane() == Pane::Search;
    let popup_active = state.popup.is_open();
    
    // Works on every pane, even search
    match (key.modifiers, key.code) {
        (X, Esc) => Some(Action::SoftReset),
        (C, Char('c')) => Some(Action::QUIT),
        (C, Char(' ')) => Some(Action::TogglePause),

        // Works on everything except search or popup
        _ if (!in_search && !popup_active) => match (key.modifiers, key.code) {
            // PLAYBACK COMMANDS
            (X, Char('`')) => Some(Action::ViewSettings),
            (X, Char(' ')) => Some(Action::TogglePause),
            (C, Char('s')) => Some(Action::Stop),
            (C, Char('n')) => Some(Action::PlayNext),
            (C, Char('p')) => Some(Action::PlayPrev),
            (X, Char('n')) => Some(Action::SeekForward(SEEK_SMALL)),
            (S, Char('N')) => Some(Action::SeekForward(SEEK_LARGE)),
            (X, Char('p')) => Some(Action::SeekBack(SEEK_SMALL)),
            (S, Char('P')) => Some(Action::SeekBack(SEEK_LARGE)),

            // NAVIGATION
            (C, Char('z')) => Some(Action::ChangeMode(Mode::Power)),
            (C, Char('t')) => Some(Action::ChangeMode(Mode::Library(LibraryView::Playlists))),
            (X, Char('/')) => Some(Action::ChangeMode(Mode::Search)),
            (C, Char('q')) => Some(Action::ChangeMode(Mode::Queue)),

            // SCROLLING
            (X, Char('j')) | (X, Down)  => Some(Action::Scroll(Director::Down(1))),
            (X, Char('k')) | (X, Up)    => Some(Action::Scroll(Director::Up(1))),
            (X, Char('d')) => Some(Action::Scroll(Director::Down(SCROLL_MID))),
            (X, Char('u')) => Some(Action::Scroll(Director::Up(SCROLL_MID))),
            (S, Char('D')) => Some(Action::Scroll(Director::Down(SCROLL_XTRA))),
            (S, Char('U')) => Some(Action::Scroll(Director::Up(SCROLL_XTRA))),
            (X, Char('g')) => Some(Action::Scroll(Director::Top)),
            (S, Char('G')) => Some(Action::Scroll(Director::Bottom)),

            (C, Char('u')) | (X, F(5))  => Some(Action::UpdateLibrary),

            _ => None,
        },
        _ => None,
    }
}

fn handle_main_pane(key: &KeyEvent, state: &UiState) -> Option<Action> {
    match (key.modifiers, key.code) {
        (X, Enter) => Some(Action::Play),

        (X, Tab) | (X, Left) | (X, Char('h')) => Some(Action::ChangeMode(Mode::Library(
            state.display_state.sidebar_view,
        ))),

        (X, Char('a')) => Some(Action::AddToPlaylist),
        (C, Char('a')) => Some(Action::GoToAlbum),


        // Queue management
        (X, Char('q')) => Some(Action::QueueSong),
        (X, Char('x')) => Some(Action::RemoveSong),
        
        // Queue entire album/playlist
        (X, Char('Q')) => {
            (state.get_mode() == Mode::Library(LibraryView::Albums)).then(|| Action::QueueEntity)
        }

        // SORTING SONGS
        (C, Left) | (C, Char('h')) => Some(Action::SortColumnsPrev),
        (C, Right) | (C, Char('l')) => Some(Action::SortColumnsNext),

        _ => None,
    }
}

fn handle_sidebar_pane(key: &KeyEvent) -> Option<Action> {
    match (key.modifiers, key.code) {
        (X, Char('q')) | (C, Enter) => Some(Action::QueueEntity),
        (X, Char('c')) => Some(Action::CreatePlaylist),
        (X, Enter) | (X, Right) | (X, Char('l')) => Some(Action::ChangePane(Pane::TrackList)),
        (X, Tab) => Some(Action::ToggleSideBar),

        // Change album sorting algorithm
        (C, Left) | (C, Char('h')) => Some(Action::ToggleAlbumSort(false)),
        (C, Right) | (C, Char('l')) => Some(Action::ToggleAlbumSort(true)),
        (C, Char('d')) => Some(Action::DeletePlaylist),

        _ => None,
    }
}

fn handle_search_pane(key: &KeyEvent) -> Option<Action> {
    match key.code {
        Tab | Char('/') | Enter => Some(Action::SendSearch),

        Char(x) if ILLEGAL_CHARS.contains(&x) => None,
        _ => Some(Action::UpdateSearch(*key)),
    }
}

fn handle_popup(key: &KeyEvent, state: &UiState) -> Option<Action> {
    if let Some(_) = state.get_error() {
        match key.code {
            Char('?') | Char('`') | Enter | Esc => Some(Action::SoftReset),
            _ => None,
        };
    }


    match &state.popup.current {
        PopupType::Settings(s)  => handle_settings(key, s),
        PopupType::Playlist(p)  => handle_playlist(key, p),
        PopupType::Error(_)     => Some(Action::ClosePopup),
        _ => None 
    }

}

fn handle_settings(key: &KeyEvent, variant: &SettingsMode ) -> Option<Action> {
    use SettingsMode::*;
    match variant {
            ViewRoots => match key.code {
                Char('a') => Some(Action::RootAdd),
                Char('d') => Some(Action::RootRemove),
                Up | Char('k') => Some(Action::PopupScrollUp),
                Down | Char('j') => Some(Action::PopupScrollDown),
                _ => None,
            },
            AddRoot => match key.code {
                Esc => Some(Action::ViewSettings),
                Enter => Some(Action::RootConfirm),
                _ => Some(Action::SettingsInput(*key)),
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
match variant {

        Create => match key.code {
            Esc => Some(Action::ClosePopup),
            Enter => Some(Action::CreatePlaylistConfirm),
            _ => Some(Action::PopupInput(*key)),
        },
        Delete => match key.code {
            Esc => Some(Action::ClosePopup),
            Enter => Some(Action::DeletePlaylistConfirm),
            _ => Some(Action::PopupInput(*key)),
        },
        AddSong => match key.code {
                Up | Char('k') => Some(Action::PopupScrollUp),
                Down | Char('j') => Some(Action::PopupScrollDown),
                Enter | Char('a') => Some(Action::AddToPlaylistConfirm),
                _ => None
        },
        Rename => todo!()
    }
}

pub fn next_event() -> Result<Option<Event>> {
    match event::poll(Duration::from_millis(REFRESH_RATE))? {
        true => Ok(Some(event::read()?)),
        false => Ok(None),
    }
}

impl Concertus {
    #[rustfmt::skip]
    pub fn handle_action(&mut self, action: Action) -> Result<()> {
        match action {
            // Player 
            Action::Play            => self.play_selected_song()?,
            Action::TogglePause     => self.player.toggle_playback()?,
            Action::Stop            => self.player.stop()?,
            Action::SeekForward(s)  => self.player.seek_forward(s)?,
            Action::SeekBack(s)     => self.player.seek_back(s)?,
            Action::PlayNext        => self.play_next()?,
            Action::PlayPrev        => self.play_prev()?,

            // UI 
            Action::Scroll(s)       => self.ui.scroll(s),
            Action::GoToAlbum       => self.ui.go_to_album()?,
            Action::ChangeMode(m)   => self.ui.set_mode(m),
            Action::ChangePane(p)   => self.ui.set_pane(p),
            Action::SortColumnsNext => self.ui.next_song_column(),
            Action::SortColumnsPrev => self.ui.prev_song_column(),
            Action::ToggleAlbumSort(next)   => self.ui.toggle_album_sort(next),
            Action::ToggleSideBar   => self.ui.toggle_sidebar_view(),

            // Search Related
            Action::UpdateSearch(k) => self.ui.process_search(k),
            Action::SendSearch      => self.ui.send_search(),

            //Playlist
            Action::CreatePlaylist  =>  if self.ui.get_sidebar_view() == &LibraryView::Playlists {
                self.ui.show_popup(PopupType::Playlist(PlaylistAction::Create));
            }

            Action::CreatePlaylistConfirm => {
                let name = self.ui.popup.input.lines()[0].clone();
                
                // Prevent duplicates 
                let playlist_names = self.ui.playlists.iter().map(|p| p.name.to_lowercase()).collect::<HashSet<_>>();

                match playlist_names.contains(&name.to_lowercase()) {
                    true => return Err(anyhow!("Playlist name already exists!")),
                    false => {
                        if let Err(e) = self.ui.create_playlist(&name) {
                            self.ui.set_error(e);
                        } else {
                            self.ui.close_popup();
                        }
                    }
                }
            }

            Action::DeletePlaylist => {
                self.ui.show_popup(PopupType::Playlist(PlaylistAction::Delete));
            }

            Action::DeletePlaylistConfirm => {
                self.ui.delete_playlist()?;
                self.ui.close_popup();
            }

            Action::AddToPlaylist => {
                self.ui.popup.selection.select_first();
                self.ui.show_popup(PopupType::Playlist(PlaylistAction::AddSong));
            }

            Action::AddToPlaylistConfirm => self.ui.add_to_playlist()?,

            Action::PopupInput(key) => {
                self.ui.popup.input.input(key);
            }

            Action::ClosePopup => self.ui.close_popup(),

            // Queue
            Action::QueueSong       => self.ui.queue_song(None)?,
            Action::QueueEntity      => self.ui.queue_entity()?,
            Action::RemoveSong      => self.ui.remove_song()?,

            // Ops
            Action::SoftReset       => self.ui.soft_reset(),
            Action::UpdateLibrary   => self.update_library()?,
            Action::QUIT            => self.ui.set_mode(Mode::QUIT),

            Action::ViewSettings    => self.activate_settings(),
            Action::PopupScrollUp      => self.popup_scroll_up(),
            Action::PopupScrollDown    => self.popup_scroll_down(),
            Action::RootAdd         => self.settings_add_root(),
            Action::RootRemove      => self.settings_remove_root(),
            Action::RootConfirm     => self.settings_root_confirm()?,

            Action::SettingsInput(key) => {
                self.ui.popup.input.input(key);
            }
            _ => (),
        }
        Ok(())
    }
}
