use super::{
    playback::PlaybackCoordinator, search_state::SearchState, theme::Theme, DisplayState,
    DisplayTheme, Pane,
};
use crate::{
    domain::{Album, Playlist, SimpleSong},
    player::PlayerState,
    ui_state::{
        popup::{PopupState, PopupType},
        LibraryView, Mode,
    },
    Library,
};
use anyhow::{Error, Result};
use ratatui::widgets::Borders;
use std::sync::{Arc, Mutex};

pub struct UiState {
    // Backend Modules
    pub(super) library: Arc<Library>,
    pub(crate) playback: PlaybackCoordinator,

    // Visual Elements
    theme: Theme,
    pub(crate) popup: PopupState,
    pub(super) search: SearchState,
    pub(crate) display_state: DisplayState,

    // View models
    pub albums: Vec<Album>,
    pub legal_songs: Vec<Arc<SimpleSong>>,
    pub playlists: Vec<Playlist>,
}

impl UiState {
    pub fn new(library: Arc<Library>, player_state: Arc<Mutex<PlayerState>>) -> Self {
        UiState {
            library,
            search: SearchState::new(),
            display_state: DisplayState::new(),
            playback: PlaybackCoordinator::new(player_state),
            popup: PopupState::new(),
            theme: Theme::set_generic_theme(),
            albums: Vec::new(),
            legal_songs: Vec::new(),
            playlists: Vec::new(),
        }
    }
}

impl UiState {
    pub fn sync_library(&mut self, library: Arc<Library>) -> Result<()> {
        self.library = library;

        self.sort_albums();
        match self.albums.is_empty() {
            true => self.display_state.album_pos.select(None),
            false => {
                let album_len = self.albums.len();
                if self.display_state.album_pos.selected().unwrap_or(0) > album_len {
                    self.display_state.album_pos.select(Some(album_len - 1));
                };
            }
        }

        self.get_playlists()?;
        self.set_legal_songs();

        Ok(())
    }

    pub fn set_error(&mut self, e: Error) {
        self.show_popup(PopupType::Error(e.to_string()));
    }

    pub fn soft_reset(&mut self) {
        if self.popup.is_open() {
            self.close_popup();
        }

        if self.get_mode() == Mode::Search {
            self.set_mode(Mode::Library(LibraryView::Albums));
        }

        self.search.input.select_all();
        self.search.input.cut();
        self.set_legal_songs();
    }

    pub fn get_theme(&self, pane: &Pane) -> DisplayTheme {
        match pane == self.get_pane() {
            true => DisplayTheme {
                // bg: Color::default(),
                bg: self.theme.bg_focused,
                border: self.theme.border_focused,
                border_display: Borders::ALL,
                text_focused: self.theme.text_focused,
                text_secondary: self.theme.text_secondary,
                text_faded: self.theme.text_unfocused,
                text_highlighted: self.theme.text_highlighted,
            },

            false => DisplayTheme {
                // bg: Color::default(),
                bg: self.theme.bg_unfocused,
                border: self.theme.border_unfocused,
                border_display: Borders::ALL,
                text_focused: self.theme.text_unfocused,
                text_secondary: self.theme.text_secondary_u,
                text_faded: self.theme.text_unfocused,
                text_highlighted: self.theme.text_highlighted_u,
            },
        }
    }

    pub fn get_error(&self) -> Option<&str> {
        match &self.popup.current {
            PopupType::Error(e) => Some(e.as_str()),
            _ => None,
        }
    }
}
