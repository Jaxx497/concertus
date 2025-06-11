use super::{
    new_textarea, playback::PlaybackCoordinator, search_state::SearchState, theme::Theme,
    AlbumDisplayItem, AlbumSort, DisplayTheme, Mode, Pane, TableSort, UiSnapshot,
};
use crate::{
    domain::{Album, QueueSong, SimpleSong, SongInfo},
    key_handler::Director,
    player::PlayerState,
    strip_win_prefix, Database, Library,
};
use anyhow::{anyhow, Context, Error, Result};
use ratatui::widgets::{Borders, ListState, TableState};
use std::{
    ops::Index,
    sync::{Arc, Mutex},
};
use tui_textarea::TextArea;

#[derive(Default, PartialEq, Clone)]
pub enum SettingsMode {
    #[default]
    ViewRoots,
    AddRoot,
    RemoveRoot,
}

pub struct UiState {
    pub(super) library: Arc<Library>,
    pub(super) mode: Mode,
    pub(super) pane: Pane,
    theme: Theme,
    table_sort: TableSort,
    album_sort: AlbumSort,
    table_pos_cached: usize,
    album_pos_cached: usize,

    pub(super) error: Option<anyhow::Error>,

    pub settings_mode: SettingsMode,
    pub settings_selection: ListState,
    pub root_input: TextArea<'static>,

    pub playback: PlaybackCoordinator,

    // These have to be public for the widgets
    // pub search: TextArea<'static>,
    pub(super) search: SearchState,
    pub table_pos: TableState,
    pub album_pos: ListState,
    pub album_display_items: Vec<AlbumDisplayItem>,
    pub legal_songs: Vec<Arc<SimpleSong>>,
    pub filtered_albums: Vec<Album>,
}

impl UiState {
    pub fn new(library: Arc<Library>, player_state: Arc<Mutex<PlayerState>>) -> Self {
        UiState {
            library,

            mode: Mode::Album,
            pane: Pane::TrackList,
            theme: Theme::set_generic_theme(),

            table_sort: TableSort::Title,
            album_sort: AlbumSort::Artist,
            table_pos_cached: 0,
            album_pos_cached: 0,

            error: None,
            playback: PlaybackCoordinator::new(player_state),

            settings_mode: SettingsMode::default(),
            settings_selection: ListState::default().with_selected(Some(0)),
            root_input: new_textarea("Enter path to directory"),

            search: SearchState::new(),
            table_pos: TableState::default().with_selected(0),
            album_pos: ListState::default().with_selected(Some(0)),
            album_display_items: Vec::new(),
            legal_songs: Vec::new(),
            filtered_albums: Vec::new(),
        }
    }
}

impl UiState {
    pub fn sync_library(&mut self, library: Arc<Library>) {
        self.library = library;

        self.sort_albums();

        match self.filtered_albums.is_empty() {
            true => self.album_pos.select(None),
            false => {
                let album_len = self.filtered_albums.len();
                if self.album_pos.selected().unwrap_or(0) > album_len {
                    self.album_pos.select(Some(album_len - 1));
                };
            }
        }

        self.set_legal_songs();
    }

    pub fn get_pane(&self) -> &Pane {
        &self.pane
    }

    pub fn set_pane(&mut self, pane: Pane) {
        self.pane = pane;
    }

    pub fn get_mode(&self) -> &Mode {
        &self.mode
    }

    pub fn set_mode(&mut self, mode: Mode) {
        match self.mode {
            Mode::Power => {
                self.table_pos_cached = self.table_pos.selected().unwrap_or(self.table_pos_cached)
            }
            Mode::Album => {
                self.album_pos_cached = self.album_pos.selected().unwrap_or(self.album_pos_cached)
            }
            _ => (),
        }

        match mode {
            Mode::Power => {
                self.mode = Mode::Power;
                self.pane = Pane::TrackList;
                self.table_sort = TableSort::Title;
                self.table_pos.select(Some(self.table_pos_cached));
            }
            Mode::Album => {
                self.mode = Mode::Album;
                self.pane = Pane::SideBar;
                match self.filtered_albums.is_empty() {
                    true => self.album_pos.select(None),
                    false => self.album_pos.select(Some(self.album_pos_cached)),
                }
                *self.table_pos.offset_mut() = 0;
                self.set_legal_songs();
            }
            Mode::Queue => {
                if !self.playback.queue.is_empty() {
                    *self.table_pos.offset_mut() = 0;
                    self.mode = Mode::Queue;
                    self.pane = Pane::TrackList;
                    self.set_legal_songs()
                }
            }
            Mode::Search => {
                self.table_sort = TableSort::Title;
                self.search.input.select_all();
                self.search.input.cut();
                self.mode = Mode::Search;
                self.pane = Pane::Search;
            }
            Mode::QUIT => {
                self.save_state().unwrap_or_else(|e| eprintln!("{e}"));

                self.mode = Mode::QUIT;
            }
        }
    }

    pub fn set_error(&mut self, e: Error) {
        self.set_pane(Pane::Popup);
        self.error = Some(e);
    }

    pub fn get_album_sort(&self) -> &AlbumSort {
        &self.album_sort
    }

    pub fn get_settings_mode(&self) -> &SettingsMode {
        &self.settings_mode
    }

    pub fn get_table_sort(&self) -> &TableSort {
        &self.table_sort
    }

    pub fn toggle_album_sort(&mut self, next: bool) {
        self.album_sort = match next {
            true => self.album_sort.next(),
            false => self.album_sort.prev(),
        };
        self.sort_albums();
        self.set_legal_songs();
    }

    fn sort_albums(&mut self) {
        self.filtered_albums = self.library.get_all_albums().to_vec();

        match self.album_sort {
            AlbumSort::Artist => self.filtered_albums.sort_by(|a, b| {
                a.artist
                    .to_lowercase()
                    .cmp(&b.artist.to_lowercase())
                    .then(a.year.cmp(&b.year))
            }),
            AlbumSort::Title => self
                .filtered_albums
                .sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase())),
            AlbumSort::Year => self.filtered_albums.sort_by(|a, b| a.year.cmp(&b.year)),
        }

        self.update_album_display_items();
    }

    fn update_album_display_items(&mut self) {
        self.album_display_items.clear();

        if self.album_sort == AlbumSort::Artist {
            let mut current_artist = None;

            for (idx, album) in self.filtered_albums.iter().enumerate() {
                let artist_str = album.artist.as_str();

                // If new artist, add a header
                if current_artist.as_ref().map_or(true, |a| a != &artist_str) {
                    self.album_display_items
                        .push(AlbumDisplayItem::Header(artist_str.to_string()));
                    current_artist = Some(artist_str);
                }

                // Add the album entry
                self.album_display_items.push(AlbumDisplayItem::Album(idx));
            }
        } else {
            // For other sort types, just add albums without headers
            for (idx, _) in self.filtered_albums.iter().enumerate() {
                self.album_display_items.push(AlbumDisplayItem::Album(idx));
            }
        }
    }

    pub fn get_album_sort_string(&self) -> String {
        self.album_sort.to_string()
    }

    pub fn go_to_album(&mut self) -> Result<()> {
        let this_song = self.get_selected_song()?;
        let this_album_title = this_song.get_album();

        self.set_mode(Mode::Album);
        self.set_pane(Pane::TrackList);

        let mut this_album = None;
        let mut album_idx = 0;
        let mut track_idx = 0;

        for (idx, album) in self.filtered_albums.iter().enumerate() {
            if album.title.as_str() == this_album_title {
                let tracklist = &album.tracklist;
                for track in tracklist {
                    if track.id == this_song.id {
                        this_album = Some(album);
                        album_idx = idx;
                        break;
                    }
                    track_idx += 1;
                }
            }
        }

        self.legal_songs = this_album.unwrap().tracklist.clone();

        // Select song and try to visually center it
        self.table_pos.select(Some(track_idx));
        *self.table_pos.offset_mut() = track_idx.checked_sub(20).unwrap_or(0);

        // Select album and try to visually center it
        self.album_pos.select(Some(album_idx));

        Ok(())
    }

    pub(crate) fn next_song_column(&mut self) {
        if self.get_search_len() < 1 {
            self.table_sort = self.table_sort.next();
            self.set_legal_songs();
        }
    }

    pub(crate) fn prev_song_column(&mut self) {
        if self.get_search_len() < 1 {
            self.table_sort = self.table_sort.prev();
            self.set_legal_songs();
        }
    }

    fn sort_by_table_column(&mut self) {
        match self.table_sort {
            TableSort::Title => {
                self.legal_songs.sort_by(|a, b| a.title.cmp(&b.title));
            }

            TableSort::Artist => self.legal_songs.sort_by(|a, b| {
                let artist_a = a.get_artist().to_lowercase();
                let artist_b = b.get_artist().to_lowercase();
                artist_a.cmp(&artist_b)
            }),
            TableSort::Album => self.legal_songs.sort_by(|a, b| {
                let album_a = a.get_album().to_lowercase();
                let album_b = b.get_album().to_lowercase();

                album_a.cmp(&album_b)
            }),
            TableSort::Duration => self.legal_songs.sort_by(|a, b| {
                a.duration
                    .partial_cmp(&b.duration)
                    .expect("Error sorting by duration.")
            }),
        };
    }

    pub fn soft_reset(&mut self) {
        match &self.error {
            Some(_) => {
                self.error = None;
                self.set_pane(Pane::TrackList);
            }
            None => {
                self.set_mode(Mode::Album);
                self.set_pane(Pane::TrackList);
                self.search.input.select_all();
                self.search.input.cut();
            }
        }
        self.set_legal_songs();
    }

    pub fn get_selected_song(&mut self) -> Result<Arc<SimpleSong>> {
        if self.legal_songs.is_empty() {
            self.table_pos.select(None);
            return Err(anyhow!("No songs to select!"));
        }

        // BUG: Using GOTO album on queue mode removes song from queue, need to fix this
        match self.mode {
            Mode::Power | Mode::Album | Mode::Search => {
                let idx = self.table_pos.selected().unwrap();
                Ok(Arc::clone(&self.legal_songs[idx]))
            }
            Mode::Queue => self
                .table_pos
                .selected()
                .and_then(|idx| self.playback.queue.remove(idx))
                .map(|s| {
                    self.set_legal_songs();
                    Arc::clone(&s.meta)
                })
                .ok_or_else(|| anyhow::anyhow!("Invalid Selection QUEUE MODE")),
            _ => Err(anyhow::anyhow!("Invalid song")),
        }
    }

    pub fn get_theme(&self, pane: &Pane) -> DisplayTheme {
        match pane == &self.pane {
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

    pub(crate) fn set_legal_songs(&mut self) {
        match &self.mode {
            Mode::Power => {
                self.legal_songs = self.library.get_all_songs().to_vec();
                self.sort_by_table_column();
            }
            Mode::Album => {
                if let Some(idx) = self.album_pos.selected() {
                    self.legal_songs = self.filtered_albums.index(idx).tracklist.clone();

                    *self.table_pos.offset_mut() = 0;
                }
            }
            Mode::Queue => {
                self.playback.queue.make_contiguous();
                self.legal_songs = self
                    .playback
                    .queue
                    .as_slices()
                    .0
                    .iter()
                    .map(|s| Arc::clone(&s.meta))
                    .collect::<Vec<Arc<_>>>();
            }
            Mode::Search => match self.get_search_len() > 1 {
                true => self.filter_songs_by_search(),
                false => {
                    self.legal_songs = self.library.get_all_songs().to_vec();
                    self.sort_by_table_column();
                }
            },
            _ => (),
        }

        // Autoselect first entry if necessary
        if !self.legal_songs.is_empty() && self.table_pos.selected().is_none() {
            self.table_pos.select(Some(0));
        }
    }
}

impl UiState {
    pub fn scroll(&mut self, director: Director) {
        match director {
            Director::Top => self.scroll_to_top(),
            Director::Bottom => self.scroll_to_bottom(),
            _ => match &self.pane {
                Pane::TrackList => self.scroll_tracklist(&director),
                Pane::SideBar => self.scroll_sidebar(&director),
                _ => (),
            },
        }
    }

    fn scroll_tracklist(&mut self, director: &Director) {
        if !self.legal_songs.is_empty() {
            let len = self.legal_songs.len();
            let selected_idx = self.table_pos.selected();

            let new_pos = match director {
                Director::Up(x) => selected_idx
                    .map(|idx| ((idx + len - (x % len)) % len + len) % len)
                    .unwrap_or(0),
                Director::Down(x) => selected_idx.map(|idx| (idx + x) % len).unwrap_or(0),
                _ => unreachable!(),
            };
            self.table_pos.select(Some(new_pos));
        }
    }

    fn scroll_sidebar(&mut self, director: &Director) {
        let base_array = &self.filtered_albums.len();

        if *base_array > 0 {
            let len = base_array;
            let selected_idx = self.album_pos.selected();
            let new_pos = selected_idx
                .map(|idx| match director {
                    Director::Up(x) => (idx + len - x) % len,
                    Director::Down(x) => (idx + x) % len,
                    _ => unreachable!(),
                })
                .unwrap_or(0);
            self.album_pos.select(Some(new_pos));
            if self.mode == Mode::Album {
                self.set_legal_songs();
            }
        }
    }

    fn scroll_to_top(&mut self) {
        match &self.pane {
            Pane::TrackList => self.table_pos.select_first(),
            Pane::SideBar => {
                match self.filtered_albums.is_empty() {
                    true => self.album_pos.select(None),
                    false => self.album_pos.select_first(),
                }
                self.set_legal_songs();
            }
            _ => (),
        }
    }

    fn scroll_to_bottom(&mut self) {
        match self.pane {
            Pane::TrackList => self.table_pos.select_last(),
            Pane::SideBar => {
                *self.table_pos.offset_mut() = 0;
                let len = self.filtered_albums.len().checked_sub(1);
                self.album_pos.select(len);
                self.set_legal_songs();
            }
            _ => (),
        }
    }
}

impl UiState {
    pub fn make_playable_song(&mut self, song: &Arc<SimpleSong>) -> Result<Arc<QueueSong>> {
        let path = self
            .library
            .get_path(song.id)
            .context("Could not retrieve path from database!")?;

        std::fs::metadata(&path).context(anyhow!(
            "Invalid file path!\n\nUnable to find: \"{}\"",
            strip_win_prefix(&path)
        ))?;

        Ok(Arc::new(QueueSong {
            meta: Arc::clone(&song),
            path,
        }))
    }
}

impl UiState {
    pub fn get_roots(&self) -> Vec<String> {
        let mut roots: Vec<String> = self
            .library
            .roots
            .iter()
            .map(|p| p.display().to_string())
            .collect();
        roots.sort();
        roots
    }

    pub fn add_root(&mut self, path: &str) -> Result<()> {
        let db = self.library.get_db();
        let mut lib = Library::init(db);
        lib.add_root(path)?;
        lib.build_library()?;

        self.library = Arc::new(lib);

        Ok(())
    }

    pub fn remove_root(&mut self) -> Result<()> {
        if let Some(selected) = self.settings_selection.selected() {
            let roots = self.get_roots();
            if selected >= roots.len() {
                return Err(anyhow!("Invalid root index!"));
            }

            let db = self.library.get_db();
            let mut lib = Library::init(db);

            let bad_root = &roots[selected];
            lib.delete_root(&bad_root)?;
        }

        Ok(())
    }

    pub fn enter_settings(&mut self) {
        self.settings_mode = SettingsMode::ViewRoots;
        if !self.get_roots().is_empty() {
            self.settings_selection.select(Some(0));
        }
        self.root_input.select_all();
        self.root_input.cut();
        self.set_pane(Pane::Popup);
    }

    pub fn create_snapshot(&self) -> UiSnapshot {
        UiSnapshot {
            mode: self.mode.to_string(),
            album_sort: self.album_sort.to_string(),
            album_selection: self.album_pos.selected(),
            song_selection: self.table_pos.selected(),
        }
    }

    pub fn save_state(&self) -> Result<()> {
        let mut db = Database::open()?;
        let snapshot = self.create_snapshot();
        db.save_ui_snapshot(&snapshot)?;
        Ok(())
    }

    pub fn restore_state(&mut self) -> Result<()> {
        let mut db = Database::open()?;

        if let Some(snapshot) = db.load_ui_snapshot()? {
            self.album_sort = AlbumSort::from_str(&snapshot.album_sort);

            self.sort_albums();

            if let Some(pos) = snapshot.album_selection {
                if pos < self.filtered_albums.len() {
                    self.album_pos.select(Some(pos));
                }
            }

            let restored_mode = Mode::from_str(&snapshot.mode);
            self.set_mode(restored_mode);

            if let Some(pos) = snapshot.song_selection {
                if pos < self.legal_songs.len() {
                    self.table_pos.select(Some(pos));
                }
            }
        }

        Ok(())
    }
}
