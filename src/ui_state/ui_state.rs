use super::{DisplayState, playback::PlaybackCoordinator, search_state::SearchState, theme::Theme};
use crate::{
    Library,
    database::DbWorker,
    domain::{Album, Playlist, SimpleSong},
    key_handler::InputContext,
    player::PlayerState,
    ui_state::{
        LibraryView, Mode, Pane,
        popup::{PopupState, PopupType},
    },
};
use anyhow::{Error, Result};
use indexmap::IndexSet;
use std::sync::{Arc, Mutex};

pub struct UiState {
    // Backend Modules
    pub(super) library: Arc<Library>,
    pub(crate) db_worker: DbWorker,
    pub(crate) playback: PlaybackCoordinator,

    // Visual Elements
    pub(crate) theme: Theme,
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
            db_worker: DbWorker::new()
                .expect("Could not establish connection to database for UiState!"),
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
                let current_selection = self.display_state.album_pos.selected().unwrap_or(0);

                if current_selection > album_len {
                    self.display_state.album_pos.select(Some(album_len - 1));
                } else if self.display_state.album_pos.selected().is_none() {
                    self.display_state.album_pos.select(Some(0));
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

        self.clear_bulk_sel();
        self.search.input.select_all();
        self.search.input.cut();
        self.set_legal_songs();
    }

    pub fn get_error(&self) -> Option<&str> {
        match &self.popup.current {
            PopupType::Error(e) => Some(e.as_str()),
            _ => None,
        }
    }

    pub fn get_bulk_sel(&self) -> &IndexSet<Arc<SimpleSong>> {
        &self.display_state.bulk_select
    }

    pub fn bulk_select_empty(&self) -> bool {
        self.display_state.bulk_select.is_empty()
    }

    pub fn clear_bulk_sel(&mut self) {
        self.display_state.bulk_select.clear();
    }

    pub fn get_input_context(&self) -> InputContext {
        if self.popup.is_open() {
            return InputContext::Popup(self.popup.current.clone());
        }

        match (self.get_mode(), self.get_pane()) {
            (Mode::Library(LibraryView::Albums), Pane::SideBar) => InputContext::AlbumView,
            (Mode::Library(LibraryView::Playlists), Pane::SideBar) => InputContext::PlaylistView,
            (Mode::Search, Pane::Search) => InputContext::Search,
            (mode, Pane::TrackList) => InputContext::TrackList(mode.clone()),
            (Mode::QUIT, _) => unreachable!(),
            _ => InputContext::TrackList(self.get_mode().clone()),
        }
    }
}
