use super::{
    playback::PlaybackCoordinator, search_state::SearchState, settings::Settings, theme::Theme,
    DisplayState, DisplayTheme, LibraryView, Mode, Pane,
};
use crate::{
    domain::{Album, Playlist, SimpleSong},
    player::PlayerState,
    Library,
};
use anyhow::Error;
use ratatui::widgets::Borders;
use std::sync::{Arc, Mutex};

pub struct UiState {
    pub(super) library: Arc<Library>,
    pub(super) search: SearchState,
    pub(crate) playback: PlaybackCoordinator,
    pub(crate) display_state: DisplayState,
    pub(crate) settings: Settings,
    theme: Theme,
    pub(super) error: Option<anyhow::Error>,

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
            settings: Settings::new(),
            theme: Theme::set_generic_theme(),
            error: None,

            albums: Vec::new(),
            legal_songs: Vec::new(),
            playlists: Vec::new(),
        }
    }
}

impl UiState {
    pub fn sync_library(&mut self, library: Arc<Library>) {
        self.library = library;

        self.sort_albums();
        match self.albums.is_empty() {
            true => self.display_state.sidebar_pos.select(None),
            false => {
                let album_len = self.albums.len();
                if self.display_state.sidebar_pos.selected().unwrap_or(0) > album_len {
                    self.display_state.sidebar_pos.select(Some(album_len - 1));
                };
            }
        }

        self.playlists = self.library.get_all_playlists().to_vec();
        self.set_legal_songs();
    }

    pub fn set_error(&mut self, e: Error) {
        self.set_pane(Pane::Popup);
        self.error = Some(e);
    }

    pub fn soft_reset(&mut self) {
        match &self.error {
            Some(_) => {
                self.error = None;
                self.set_pane(Pane::TrackList);
            }
            None => {
                self.set_mode(Mode::Library(LibraryView::Albums));
                self.set_pane(Pane::TrackList);
                self.search.input.select_all();
                self.search.input.cut();
            }
        }
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
                border_display: Borders::NONE,
                text_focused: self.theme.text_unfocused,
                text_secondary: self.theme.text_secondary_u,
                text_faded: self.theme.text_unfocused,
                text_highlighted: self.theme.text_highlighted_u,
            },
        }
    }

    pub fn get_error(&self) -> &Option<Error> {
        &self.error
    }
}
