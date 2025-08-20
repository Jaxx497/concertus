use super::{AlbumDisplayItem, AlbumSort, LibraryView, Mode, Pane, TableSort, UiState};
use crate::{
    domain::{Album, SimpleSong, SongInfo},
    key_handler::Director,
};
use anyhow::{anyhow, Result};
use ratatui::widgets::{ListState, TableState};
use std::{ops::Index, sync::Arc};

pub struct DisplayState {
    mode: Mode,
    pub pane: Pane,

    table_sort: TableSort,
    pub(super) album_sort: AlbumSort,

    pub sidebar_view: LibraryView,
    pub sidebar_pos: ListState, // Active Position
    pub table_pos: TableState,

    // Specific States
    album_selection: Option<usize>,
    playlist_selection: Option<usize>,

    table_pos_cached: usize,

    pub album_headers: Vec<AlbumDisplayItem>,
}

impl DisplayState {
    pub fn new() -> Self {
        DisplayState {
            mode: Mode::Library(LibraryView::Albums),
            pane: Pane::TrackList,

            table_sort: TableSort::Title,
            album_sort: AlbumSort::Artist,

            sidebar_view: LibraryView::Albums,

            table_pos: TableState::default().with_selected(0),
            sidebar_pos: ListState::default().with_selected(Some(0)),

            table_pos_cached: 0,
            album_selection: Some(0),
            playlist_selection: Some(0),

            album_headers: Vec::new(),
        }
    }
}

impl UiState {
    pub fn get_pane(&self) -> &Pane {
        &self.display_state.pane
    }

    pub fn set_pane(&mut self, pane: Pane) {
        self.display_state.pane = pane;
    }

    pub fn get_mode(&self) -> &Mode {
        &self.display_state.mode
    }

    pub fn get_sidebar_view(&self) -> &LibraryView {
        &self.display_state.sidebar_view
    }

    pub fn set_mode(&mut self, mode: Mode) {
        match self.display_state.mode {
            Mode::Power => {
                self.display_state.table_pos_cached = self
                    .display_state
                    .table_pos
                    .selected()
                    .unwrap_or(self.display_state.table_pos_cached)
            }
            Mode::Library(LibraryView::Albums) => {
                self.display_state.album_selection = self.display_state.sidebar_pos.selected()
            }
            Mode::Library(LibraryView::Playlists) => {
                self.display_state.playlist_selection = self.display_state.sidebar_pos.selected()
            }
            _ => (),
        }

        match mode {
            Mode::Power => {
                self.set_legal_songs();
                self.display_state.mode = Mode::Power;
                self.display_state.pane = Pane::TrackList;
                self.display_state.table_sort = TableSort::Title;
                self.display_state
                    .table_pos
                    .select(Some(self.display_state.table_pos_cached));
            }
            Mode::Library(LibraryView::Albums) => {
                self.display_state.sidebar_view = LibraryView::Albums;
                self.display_state.mode = Mode::Library(self.display_state.sidebar_view);
                self.display_state.pane = Pane::SideBar;
                match self.albums.is_empty() {
                    true => self.display_state.sidebar_pos.select(None),
                    false => self
                        .display_state
                        .sidebar_pos
                        .select(self.display_state.album_selection),
                }
                *self.display_state.table_pos.offset_mut() = 0;
                self.set_legal_songs();
            }

            Mode::Library(LibraryView::Playlists) => {
                self.display_state.sidebar_view = LibraryView::Playlists;
                self.display_state.mode = Mode::Library(self.display_state.sidebar_view);
                self.display_state.pane = Pane::SideBar;
            }
            Mode::Queue => {
                if !self.queue_is_empty() {
                    *self.display_state.table_pos.offset_mut() = 0;
                    self.display_state.mode = Mode::Queue;
                    self.display_state.pane = Pane::TrackList;
                    self.set_legal_songs()
                }
            }
            Mode::Search => {
                self.display_state.table_sort = TableSort::Title;
                self.search.input.select_all();
                self.search.input.cut();
                self.display_state.mode = Mode::Search;
                self.display_state.pane = Pane::Search;
            }
            Mode::QUIT => {
                self.save_state().unwrap_or_else(|e| eprintln!("{e}"));
                let _ = self
                    .library
                    .set_history_db(&self.playback.history.make_contiguous());
                self.display_state.mode = Mode::QUIT;
            }
        }
    }

    pub fn get_selected_song(&mut self) -> Result<Arc<SimpleSong>> {
        if self.legal_songs.is_empty() {
            self.display_state.table_pos.select(None);
            return Err(anyhow!("No songs to select!"));
        }

        // BUG: Using GOTO album on queue mode removes song from queue, need to fix this
        match self.display_state.mode {
            Mode::Power | Mode::Library(LibraryView::Albums) | Mode::Search => {
                let idx = self.display_state.table_pos.selected().unwrap();
                Ok(Arc::clone(&self.legal_songs[idx]))
            }
            Mode::Queue => self
                .display_state
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

    pub fn get_selected_album(&self) -> Option<&Album> {
        self.display_state
            .sidebar_pos
            .selected()
            .and_then(|idx| self.albums.get(idx))
    }

    pub fn get_album_sort(&self) -> &AlbumSort {
        &self.display_state.album_sort
    }

    pub fn get_table_sort(&self) -> &TableSort {
        &self.display_state.table_sort
    }

    pub fn toggle_sidebar_view(&mut self) {
        self.display_state.sidebar_view = match self.display_state.sidebar_view {
            LibraryView::Albums => LibraryView::Playlists,
            LibraryView::Playlists => LibraryView::Albums,
        };

        self.set_mode(Mode::Library(self.display_state.sidebar_view));
        //self.update_sidebar_items();
        self.set_legal_songs();
    }

    pub fn toggle_album_sort(&mut self, next: bool) {
        self.display_state.album_sort = match next {
            true => self.display_state.album_sort.next(),
            false => self.display_state.album_sort.prev(),
        };
        self.sort_albums();
        self.set_legal_songs();
    }

    pub(super) fn sort_albums(&mut self) {
        self.albums = self.library.get_all_albums().to_vec();

        match self.display_state.album_sort {
            AlbumSort::Artist => self.albums.sort_by(|a, b| {
                a.artist
                    .to_lowercase()
                    .cmp(&b.artist.to_lowercase())
                    .then(a.year.cmp(&b.year))
            }),
            AlbumSort::Title => self
                .albums
                .sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase())),
            AlbumSort::Year => self.albums.sort_by(|a, b| a.year.cmp(&b.year)),
        }

        self.update_sidebar_view();
    }

    fn update_sidebar_view(&mut self) {
        self.display_state.album_headers.clear();

        match self.display_state.album_sort {
            AlbumSort::Artist => {
                let mut current_artist = None;
                for (idx, album) in self.albums.iter().enumerate() {
                    let artist_str = album.artist.as_str();

                    // If new artist, add a header
                    if current_artist.as_ref().map_or(true, |a| a != &artist_str) {
                        self.display_state
                            .album_headers
                            .push(AlbumDisplayItem::Header(artist_str.to_string()));
                        current_artist = Some(artist_str);
                    }

                    // Add the album entry
                    self.display_state
                        .album_headers
                        .push(AlbumDisplayItem::Album(idx));
                }
            }
            _ => {
                for (idx, _) in self.albums.iter().enumerate() {
                    self.display_state
                        .album_headers
                        .push(AlbumDisplayItem::Album(idx));
                }
            }
        }
    }

    pub fn get_album_sort_string(&self) -> String {
        self.display_state.album_sort.to_string()
    }

    pub(crate) fn next_song_column(&mut self) {
        if self.get_search_len() < 1 {
            self.display_state.table_sort = self.display_state.table_sort.next();
            self.set_legal_songs();
        }
    }

    pub(crate) fn prev_song_column(&mut self) {
        if self.get_search_len() < 1 {
            self.display_state.table_sort = self.display_state.table_sort.prev();
            self.set_legal_songs();
        }
    }

    fn sort_by_table_column(&mut self) {
        match self.display_state.table_sort {
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

    pub(crate) fn go_to_album(&mut self) -> Result<()> {
        let this_song = self.get_selected_song()?;
        let this_album_title = this_song.get_album();

        self.set_mode(Mode::Library(LibraryView::Albums));
        self.set_pane(Pane::TrackList);

        let mut this_album = None;
        let mut album_idx = 0;
        let mut track_idx = 0;

        for (idx, album) in self.albums.iter().enumerate() {
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
        self.display_state.table_pos.select(Some(track_idx));
        *self.display_state.table_pos.offset_mut() = track_idx.checked_sub(20).unwrap_or(0);

        // Select album and try to visually center it
        self.display_state.sidebar_pos.select(Some(album_idx));

        Ok(())
    }

    pub(crate) fn set_legal_songs(&mut self) {
        match &self.display_state.mode {
            Mode::Power => {
                self.legal_songs = self.library.get_all_songs().to_vec();
                self.sort_by_table_column();
            }
            Mode::Library(_) => {
                // Mode::Library(LibraryView::Albums) => {
                if let Some(idx) = self.display_state.sidebar_pos.selected() {
                    self.legal_songs = self.albums.index(idx).tracklist.clone();
                    *self.display_state.table_pos.offset_mut() = 0;
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

        // Autoselect first entry if table_pos selection is none
        if !self.legal_songs.is_empty() && self.display_state.table_pos.selected().is_none() {
            self.display_state.table_pos.select(Some(0));
        }
    }
}

impl UiState {
    pub fn scroll(&mut self, director: Director) {
        match director {
            Director::Top => self.scroll_to_top(),
            Director::Bottom => self.scroll_to_bottom(),
            _ => match &self.display_state.pane {
                Pane::TrackList => self.scroll_tracklist(&director),
                Pane::SideBar => self.scroll_sidebar(&director),
                _ => (),
            },
        }
    }

    fn scroll_tracklist(&mut self, director: &Director) {
        if !self.legal_songs.is_empty() {
            let len = self.legal_songs.len();
            let selected_idx = self.display_state.table_pos.selected();

            let new_pos = match director {
                Director::Up(x) => selected_idx
                    .map(|idx| ((idx + len - (x % len)) % len + len) % len)
                    .unwrap_or(0),
                Director::Down(x) => selected_idx.map(|idx| (idx + x) % len).unwrap_or(0),
                _ => unreachable!(),
            };
            self.display_state.table_pos.select(Some(new_pos));
        }
    }

    fn scroll_sidebar(&mut self, director: &Director) {
        let base_array = &self.albums.len();

        if *base_array > 0 {
            let len = base_array;
            let selected_idx = self.display_state.sidebar_pos.selected();
            let new_pos = selected_idx
                .map(|idx| match director {
                    Director::Up(x) => (idx + len - x) % len,
                    Director::Down(x) => (idx + x) % len,
                    _ => unreachable!(),
                })
                .unwrap_or(0);
            self.display_state.sidebar_pos.select(Some(new_pos));
            if self.display_state.mode == Mode::Library(LibraryView::Albums) {
                self.set_legal_songs();
            }
        }
    }

    fn scroll_to_top(&mut self) {
        match &self.display_state.pane {
            Pane::TrackList => self.display_state.table_pos.select_first(),
            Pane::SideBar => {
                match self.albums.is_empty() {
                    true => self.display_state.sidebar_pos.select(None),
                    false => self.display_state.sidebar_pos.select_first(),
                }
                self.set_legal_songs();
            }
            _ => (),
        }
    }

    fn scroll_to_bottom(&mut self) {
        match self.display_state.pane {
            Pane::TrackList => self.display_state.table_pos.select_last(),
            Pane::SideBar => {
                *self.display_state.table_pos.offset_mut() = 0;
                let len = self.albums.len().checked_sub(1);
                self.display_state.sidebar_pos.select(len);
                self.set_legal_songs();
            }
            _ => (),
        }
    }
}
