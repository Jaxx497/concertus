use super::{
    theme::Theme, AlbumDisplayItem, AlbumSort, DisplayTheme, Mode, Pane, TableSort,
    HISTORY_CAPACITY, MATCHER, MATCH_THRESHOLD,
};
use crate::{
    domain::{Album, QueueSong, SimpleSong, SongInfo},
    key_handler::Director,
    library,
    player::{PlaybackState, PlayerState},
    strip_win_prefix, Library,
};
use anyhow::{anyhow, Context, Error, Result};
use fuzzy_matcher::FuzzyMatcher;
use ratatui::{
    crossterm::event::KeyEvent,
    style::Style,
    widgets::{ListState, TableState},
};
use std::{
    collections::VecDeque,
    ops::Index,
    sync::{Arc, Mutex},
    time::Duration,
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
    library: Arc<Library>,
    player_state: Arc<Mutex<PlayerState>>,

    mode: Mode,
    pane: Pane,
    theme: Theme,
    table_sort: TableSort,
    album_sort: AlbumSort,
    table_pos_cached: usize,
    album_pos_cached: usize,

    waveform: Vec<f32>,

    error: Option<anyhow::Error>,

    pub settings_mode: SettingsMode,
    pub settings_selection: usize,
    pub new_root_input: TextArea<'static>,

    pub queue: VecDeque<Arc<QueueSong>>,
    pub history: VecDeque<Arc<SimpleSong>>,

    // These have to be public for the widgets
    pub search: TextArea<'static>,
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
            player_state,

            mode: Mode::Power,
            pane: Pane::TrackList,
            theme: Theme::set_generic_theme(),
            table_sort: TableSort::Title,
            album_sort: AlbumSort::Artist,
            table_pos_cached: 0,
            album_pos_cached: 0,

            waveform: Vec::new(),

            error: None,
            queue: VecDeque::new(),
            history: VecDeque::new(),

            settings_mode: SettingsMode::default(),
            settings_selection: 0,
            new_root_input: new_root_input_textarea(),

            search: new_textarea(),
            table_pos: TableState::default()
                .with_selected(Some(0))
                .with_selected_column(Some(0)),
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

    pub fn update_player_state(&mut self, player_state: Arc<Mutex<PlayerState>>) {
        self.player_state = player_state
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
                *self.table_pos.offset_mut() = self.table_pos_cached.checked_sub(10).unwrap_or(0);
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
                if !self.queue.is_empty() {
                    *self.table_pos.offset_mut() = 0;
                    self.mode = Mode::Queue;
                    self.pane = Pane::TrackList;
                    self.set_legal_songs()
                }
            }
            Mode::Search => {
                self.table_sort = TableSort::Title;
                self.search.select_all();
                self.search.cut();
                self.mode = Mode::Search;
                self.pane = Pane::Search;
            }
            Mode::QUIT => self.mode = Mode::QUIT,
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

    pub fn check_player_error(&mut self) {
        let mut state = self.player_state.lock().unwrap();

        if let Some(e) = state.player_error.take() {
            self.error = Some(e);
            self.pane = Pane::Popup;
        }
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

    pub(crate) fn sort_albums(&mut self) {
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
                self.set_mode(Mode::Power);
                self.search.select_all();
                self.search.cut();
            }
        }
        self.set_legal_songs();
    }

    pub fn get_selected_song(&mut self) -> Result<Arc<SimpleSong>> {
        if self.legal_songs.is_empty() {
            self.table_pos.select(None);
            return Err(anyhow!("No songs to select!"));
        }

        match self.mode {
            Mode::Power | Mode::Album | Mode::Search => {
                let idx = self.table_pos.selected().unwrap();
                Ok(Arc::clone(&self.legal_songs[idx]))
            }
            Mode::Queue => self
                .table_pos
                .selected()
                .and_then(|idx| self.queue.remove(idx))
                .map(|s| {
                    self.set_legal_songs();
                    Arc::clone(&s.meta)
                })
                .ok_or_else(|| anyhow::anyhow!("Invalid Selection QUEUE MODE")),
            _ => Err(anyhow::anyhow!("Invalid song")),
        }
    }

    pub fn get_selected_album_title(&self) -> &str {
        let idx = self.album_pos.selected().unwrap_or(0);
        &self.filtered_albums[idx].title
    }

    pub fn get_theme(&self, pane: &Pane) -> DisplayTheme {
        match pane == &self.pane {
            true => DisplayTheme {
                bg: self.theme.bg_focused,
                border_type: self.theme.border_type,
                border: self.theme.border_focused,
                text_focused: self.theme.text_focused,
                text_secondary: self.theme.text_secondary,
                text_faded: self.theme.text_unfocused,
                text_highlighted: self.theme.text_highlighted,
            },

            false => DisplayTheme {
                bg: self.theme.bg_unfocused,
                border_type: self.theme.border_type,
                border: self.theme.border_unfocused,
                text_focused: self.theme.text_unfocused,
                text_secondary: self.theme.text_unfocused,
                text_faded: self.theme.text_unfocused,
                text_highlighted: self.theme.text_unfocused,
            },
        }
    }

    pub fn get_error(&self) -> &Option<Error> {
        &self.error
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

            let (new_pos, scroll_amount) = match director {
                Director::Up(x) => {
                    let new_pos = selected_idx
                        .map(|idx| ((idx + len - (x % len)) % len + len) % len)
                        .unwrap_or(0);
                    (new_pos, *x)
                }
                Director::Down(x) => {
                    let new_pos = selected_idx.map(|idx| (idx + x) % len).unwrap_or(0);
                    (new_pos, *x)
                }
                _ => unreachable!(),
            };
            self.table_pos.select(Some(new_pos));

            if scroll_amount > 1 {
                *self.table_pos.offset_mut() = new_pos.checked_sub(15).unwrap_or(0);
            }
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

// SEARCH RELATED
impl UiState {
    pub fn get_search_widget(&mut self) -> &mut TextArea<'static> {
        &mut self.search
    }

    pub fn get_search_len(&self) -> usize {
        self.search.lines()[0].len()
    }

    pub fn send_search(&mut self) {
        match !self.legal_songs.is_empty() {
            true => self.set_pane(Pane::TrackList),
            false => self.soft_reset(),
        }
    }

    pub fn process_search(&mut self, k: KeyEvent) {
        self.search.input(k);
        self.set_legal_songs();
        if self.legal_songs.is_empty() {
            self.table_pos.select(None);
        } else {
            self.table_pos.select(Some(0));
        }
    }

    pub fn read_search(&self) -> &str {
        &self.search.lines()[0]
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
                }
            }
            Mode::Queue => {
                self.queue.make_contiguous();
                self.legal_songs = self
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

    fn filter_songs_by_search(&mut self) {
        let query = self.read_search().to_lowercase();

        let mut scored_songs: Vec<(Arc<SimpleSong>, i64)> = self
            .library
            .get_all_songs()
            .iter()
            .filter_map(|song| {
                MATCHER
                    .fuzzy_match(&song.get_title().to_lowercase(), &query.as_str())
                    .filter(|&score| score > MATCH_THRESHOLD)
                    .map(|score| (song.clone(), score))
            })
            .collect();

        scored_songs.sort_by(|a, b| b.1.cmp(&a.1));

        self.legal_songs = scored_songs.into_iter().map(|i| i.0).collect();
    }
}

// Player Related
impl UiState {
    pub fn get_now_playing(&self) -> Option<Arc<SimpleSong>> {
        let state = self.player_state.lock().unwrap();
        state.now_playing.clone()
    }

    pub fn get_playback_elapsed(&self) -> Duration {
        let state = self.player_state.lock().unwrap();
        state.elapsed
    }

    pub fn is_not_playing(&self) -> bool {
        let state = self.player_state.lock().unwrap();
        state.state == PlaybackState::Stopped
    }

    pub fn is_paused(&self) -> bool {
        let state = self.player_state.lock().unwrap();
        state.state == PlaybackState::Paused
    }

    pub fn get_waveform(&self) -> &[f32] {
        self.waveform.as_slice()
    }

    pub fn clear_waveform(&mut self) {
        self.waveform.clear();
    }

    pub fn set_waveform(&mut self, wf: Vec<f32>) {
        self.waveform = wf
    }
}

impl UiState {
    pub fn queue_song(&mut self, song: Option<Arc<SimpleSong>>) -> Result<()> {
        let simple_song = match song {
            Some(s) => s,
            None => self.get_selected_song()?,
        };

        let queue_song = self.make_playable_song(&simple_song)?;

        self.queue.push_back(queue_song);

        Ok(())
    }

    pub fn queue_album(&mut self) -> Result<()> {
        let album = self
            .album_pos
            .selected()
            .ok_or_else(|| anyhow::anyhow!("Illegal album selection!"))?;

        let songs = self.filtered_albums.index(album).tracklist.clone();

        for song in songs {
            self.queue_song(Some(song))?;
        }

        Ok(())
    }

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

    pub fn peek_queue(&self) -> Option<&Arc<SimpleSong>> {
        self.queue.front().map(|q| &q.meta)
    }

    pub fn queue_is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn remove_from_queue(&mut self) -> Result<()> {
        if Mode::Queue == self.mode {
            self.table_pos
                .selected()
                .and_then(|idx| self.queue.remove(idx))
                .map(|_| {
                    self.set_legal_songs();
                });
        }
        Ok(())
    }

    pub(crate) fn load_history(&mut self) {
        self.history = self
            .library
            .load_history(&self.legal_songs)
            .unwrap_or_default();
    }

    pub fn add_to_history(&mut self, song: Arc<SimpleSong>) {
        if let Some(last) = self.history.front() {
            if last.id == song.id {
                return;
            }
        }

        self.history.push_front(song);

        while self.history.len() > HISTORY_CAPACITY {
            self.history.pop_back();
        }
    }

    pub fn get_prev_song(&mut self) -> Option<Arc<SimpleSong>> {
        match self.get_now_playing() {
            Some(_) => self.history.remove(1),
            None => self.history.remove(0),
        }
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

    pub fn remove_root(&mut self, index: usize) -> Result<()> {
        let roots = self.get_roots();
        if index >= roots.len() {
            return Err(anyhow!("Invalid root index!"));
        }

        let db = self.library.get_db();
        let mut lib = Library::init(db);

        let bad_root = &roots[index];
        lib.delete_root(&bad_root)?;

        Ok(())
    }

    pub fn enter_settings(&mut self) {
        self.settings_mode = SettingsMode::ViewRoots;
        self.settings_selection = 0;
        self.new_root_input.select_all();
        self.new_root_input.cut();
        self.set_pane(Pane::Popup);
    }
}

fn new_textarea() -> TextArea<'static> {
    let mut search = TextArea::default();
    search.set_cursor_line_style(Style::default());
    search.set_placeholder_text(" Enter search term: ");

    search
}

fn new_root_input_textarea() -> TextArea<'static> {
    let mut input = TextArea::default();
    input.set_cursor_line_style(Style::default());
    input.set_placeholder_text(" Enter directory path: ");

    input
}
