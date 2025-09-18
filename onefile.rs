// Project: concertus (v0.0.7)

// .\src\app_core\app.rs
use crate::{
    Library,
    domain::{QueueSong, SongDatabase as _, SongInfo, generate_waveform},
    key_handler::{self},
    overwrite_line,
    player::PlayerController,
    tui,
    ui_state::{Mode, PopupType, SettingsMode, UiState},
};
use anyhow::{Result, anyhow};
use ratatui::crossterm::event::{Event, KeyEventKind};
use std::{
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver},
    },
    thread,
    time::{Duration, Instant},
};

pub struct Concertus {
    _initializer: Instant,
    library: Arc<Library>,
    pub(crate) ui: UiState,
    pub(crate) player: PlayerController,
    waveform_rec: Option<Receiver<Vec<f32>>>,
    requires_setup: bool,
}

impl Concertus {
    pub fn new() -> Self {
        let lib = Library::init();
        let lib = Arc::new(lib);
        let lib_clone = Arc::clone(&lib);

        let shared_state = Arc::new(Mutex::new(crate::player::PlayerState::default()));
        let shared_state_clone = Arc::clone(&shared_state);

        Concertus {
            _initializer: Instant::now(),
            library: lib,
            player: PlayerController::new(),
            ui: UiState::new(lib_clone, shared_state_clone),
            waveform_rec: None,
            requires_setup: true,
        }
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let mut terminal = ratatui::init();
        terminal.clear()?;

        self.preload_lib();
        self.initialize_ui();

        if self.requires_setup {
            self.ui
                .show_popup(PopupType::Settings(SettingsMode::AddRoot));
            self.ui.display_state.album_pos.select_first();
        }

        // MAIN ROUTINE
        loop {
            self.ui.update_player_state(self.player.get_shared_state());

            // Check for user input
            match key_handler::next_event()? {
                Some(Event::Key(key)) if key.kind == KeyEventKind::Press => {
                    if let Some(action) = key_handler::handle_key_event(key, &self.ui) {
                        if let Err(e) = self.handle_action(action) {
                            self.ui.set_error(e);
                        }
                    }
                }
                _ => (),
            }

            // Play next song if song in queue and current song has ended
            if self.ui.is_not_playing() {
                if !self.ui.queue_is_empty() {
                    if let Some(song) = self.ui.playback.queue.pop_front() {
                        if let Err(e) = self.play_song(song) {
                            self.ui.set_error(e);
                        };
                        thread::sleep(Duration::from_millis(75)); // Prevents flickering on waveform widget during song change
                    }
                }
                self.ui.set_legal_songs();
            }

            let _ = self.await_waveform_completion();

            terminal.draw(|f| tui::render(f, &mut self.ui))?;

            if self.ui.get_mode() == Mode::QUIT {
                self.player.stop()?;
                break;
            }
        }
        ratatui::restore();
        overwrite_line("Shutting down... do not close terminal!");
        overwrite_line("Thank you for using concertus!\n\n");

        Ok(())
    }

    fn _debug_startup(&self) {
        let finisher = (Instant::now() - self._initializer).as_secs_f32();
        println!("Finished initializing in {finisher}");
    }

    pub fn preload_lib(&mut self) {
        let mut updated_lib = Library::init();

        if !updated_lib.roots.is_empty() {
            self.requires_setup = false
        };

        // TODO: MAKE THIS OPTIONAL
        // updated_lib.update_db().unwrap();
        updated_lib.build_library().unwrap();

        self.library = Arc::new(updated_lib);
        if let Err(e) = self.ui.sync_library(Arc::clone(&self.library)) {
            self.ui.set_error(e);
        }
    }

    pub fn initialize_ui(&mut self) {
        self.ui.soft_reset();
        self.ui.load_history();
        let _ = self.ui.restore_state();
    }
}

impl Concertus {
    fn play_song(&mut self, song: Arc<QueueSong>) -> Result<()> {
        // Return from function early if selected song is already playing
        if let Some(now_playing) = self.ui.get_now_playing() {
            if now_playing.id == song.get_id() {
                return Ok(());
            }
        }

        if !std::fs::metadata(&song.path).is_ok() {
            return Err(anyhow!("File not found: {}", &song.path));
        }

        self.ui.clear_waveform();
        self.waveform_handler(&song)?;
        song.update_play_count()?;
        self.player.play_song(song)?;

        Ok(())
    }

    pub(crate) fn play_selected_song(&mut self) -> Result<()> {
        let song = self.ui.get_selected_song()?;

        if self.ui.get_mode() == &Mode::Queue {
            self.ui.remove_song()?;
        }

        let queue_song = self.ui.make_playable_song(&song)?;

        self.ui.add_to_history(Arc::clone(&song));
        self.play_song(queue_song)
    }

    pub(crate) fn play_next(&mut self) -> Result<()> {
        match self.ui.playback.queue.pop_front() {
            Some(song) => {
                self.ui.add_to_history(Arc::clone(&song.meta));
                self.play_song(song)?;
            }
            None => self.player.stop()?,
        }
        self.ui.set_legal_songs();

        Ok(())
    }

    pub(crate) fn play_prev(&mut self) -> Result<()> {
        match self.ui.get_prev_song() {
            Some(prev) => {
                if let Some(now_playing) = self.ui.get_now_playing() {
                    let queue_song = self.ui.make_playable_song(&now_playing)?;
                    self.ui.playback.queue.push_front(queue_song);
                }
                let queue_song = self.ui.make_playable_song(&prev)?;
                self.play_song(queue_song)?;
            }
            None => self.ui.set_error(anyhow!("End of history!")),
        }

        self.ui.set_legal_songs();
        Ok(())
    }
}

impl Concertus {
    fn waveform_handler(&mut self, song: &QueueSong) -> Result<()> {
        let path_clone = song.path.clone();

        match song.get_waveform() {
            Ok(wf) => self.ui.set_waveform(wf),
            _ => {
                let (tx, rx) = mpsc::channel();

                thread::spawn(move || {
                    let waveform = generate_waveform(&path_clone);
                    let _ = tx.send(waveform);
                });
                self.waveform_rec = Some(rx);
            }
        };
        Ok(())
    }

    fn await_waveform_completion(&mut self) -> Result<()> {
        if self.ui.get_waveform_visual().is_empty() && self.ui.get_now_playing().is_some() {
            if let Some(rx) = &self.waveform_rec {
                if let Ok(waveform) = rx.try_recv() {
                    let song = self.player.get_now_playing().unwrap();

                    song.set_waveform(&waveform)?;
                    self.ui.set_waveform(waveform);
                    self.waveform_rec = None;
                    return Ok(());
                }
            }
            return Err(anyhow!("Invalid waveform"));
        }
        Ok(())
    }

    pub(crate) fn update_library(&mut self) -> Result<()> {
        let mut updated_lib = Library::init();

        let cached = self.ui.display_state.album_pos.selected();
        self.ui.display_state.album_pos.select(None);

        // TODO: Alert user of changes on update
        updated_lib.build_library()?;

        let updated_len = updated_lib.albums.len();

        self.library = Arc::new(updated_lib);
        if let Err(e) = self.ui.sync_library(Arc::clone(&self.library)) {
            self.ui.set_error(e);
        }

        // Do not index a value out of bounds if current selection
        // will be out of bounds after update

        if updated_len > 0 {
            self.ui
                .display_state
                .album_pos
                .select(match cached < Some(updated_len) {
                    true => cached,
                    false => Some(updated_len / 2),
                })
        }

        self.ui.set_legal_songs();

        Ok(())
    }
}

// .\src\app_core\mod.rs
mod app;
pub use app::Concertus;

// .\src\database\mod.rs
use crate::domain::{LongSong, SimpleSong, SongInfo};
use anyhow::Result;
use indexmap::IndexMap;
use queries::*;
use rusqlite::{Connection, params};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs,
    path::PathBuf,
    sync::Arc,
    time::{Duration, UNIX_EPOCH},
};

mod playlists;
mod queries;
mod snapshot;
mod tables;
mod worker;

pub use worker::DbWorker;

const CONFIG_DIRECTORY: &'static str = "Concertus";
const DATABASE_FILENAME: &'static str = "concertus.db";

pub struct Database {
    conn: Connection,
    artist_map: HashMap<i64, Arc<String>>,
    album_map: HashMap<i64, Arc<String>>,
}

impl Database {
    pub fn open() -> Result<Self> {
        let db_path = dirs::config_dir()
            .expect("Config folder not present on system!")
            .join(CONFIG_DIRECTORY);

        fs::create_dir_all(&db_path).expect("Failed to create or access config directory");

        let conn = Connection::open(db_path.join(DATABASE_FILENAME))?;

        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "cache_size", "1000")?;

        let mut db = Database {
            conn,
            artist_map: HashMap::new(),
            album_map: HashMap::new(),
        };
        db.create_tables()?;

        Ok(db)
    }

    fn create_tables(&mut self) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute_batch(tables::CREATE_TABLES)?;
        tx.commit()?;

        Ok(())
    }

    // ===================
    //   SONG OPERATIONS
    // ===================

    pub(crate) fn insert_songs(&mut self, song_list: &[LongSong]) -> Result<()> {
        let artist_map = self.get_artist_map_name_to_id()?;
        let album_map = self.get_album_map_name_to_id()?;

        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare_cached(INSERT_SONG)?;

            for song in song_list {
                // Get artist ID for the song's artist
                let artist_id = artist_map.get(song.get_artist()).cloned();

                // Get artist ID for the album artist
                let album_artist_id = artist_map.get(song.album_artist.as_str()).cloned();

                // Look up album ID using both title and album artist ID
                let album_id = album_artist_id
                    .and_then(|aid| album_map.get(&(song.get_album().to_string(), aid)).cloned());

                if artist_id.is_none() || album_id.is_none() {
                    eprintln!(
                        "Skipping song {}: artist_id={:?}, album_id={:?}",
                        song.title, artist_id, album_id
                    );
                    continue;
                }

                stmt.execute(params![
                    song.id.to_le_bytes(),
                    &song.title,
                    &song.year,
                    &song.path.to_str(),
                    artist_id.unwrap(),
                    album_id.unwrap(),
                    &song.track_no,
                    &song.disc_no,
                    &song.duration.as_secs_f32(),
                    &song.sample_rate,
                    &song.filetype
                ])?;
            }
        }
        tx.commit()?;

        Ok(())
    }

    pub(crate) fn get_all_songs(&mut self) -> Result<IndexMap<u64, Arc<SimpleSong>>> {
        self.set_album_map()?;
        self.set_artist_map()?;

        let mut stmt = self.conn.prepare(GET_ALL_SONGS)?;

        let songs = stmt
            .query_map([], |row| {
                let hash_bytes: Vec<u8> = row.get("id")?;
                let hash_array: [u8; 8] = hash_bytes.try_into().expect("Invalid hash bytes length");
                let hash = u64::from_le_bytes(hash_array);

                let artist_id = row.get("artist_id")?;
                let album_artist_id = row.get("album_artist")?;

                let artist = match self.artist_map.get(&artist_id) {
                    Some(a) => Arc::clone(a),
                    None => Arc::new(format!("Unknown Artist")),
                };

                let album_artist = match self.artist_map.get(&album_artist_id) {
                    Some(a) => Arc::clone(a),
                    None => Arc::new(format!("Unknown Artist")),
                };

                let album_id = row.get("album_id")?;
                let album = match self.album_map.get(&album_id) {
                    Some(a) => Arc::clone(a),
                    None => Arc::new(format!("Unknown Album")),
                };

                let song = SimpleSong {
                    id: hash,
                    title: row.get("title")?,
                    artist,
                    album,
                    album_artist,
                    year: row.get("year")?,
                    track_no: row.get("track_no")?,
                    disc_no: row.get("disc_no")?,
                    duration: Duration::from_secs_f32(row.get("duration")?),
                    filetype: row.get("format")?,
                };

                Ok((hash, Arc::new(song)))
            })?
            .filter_map(Result::ok)
            .collect();

        Ok(songs)
    }

    pub(crate) fn delete_songs(&mut self, to_delete: &[u64]) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare(DELETE_SONGS)?;
            for id in to_delete {
                stmt.execute([id.to_le_bytes()])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub(crate) fn update_play_count(&mut self, id: u64) -> Result<()> {
        let id = id.to_le_bytes();
        self.conn.execute(UPDATE_PLAY_COUNT, params![id, 1])?;

        Ok(())
    }

    pub(crate) fn get_song_path(&mut self, id: u64) -> Result<String> {
        let output = self
            .conn
            .query_row(GET_PATH, [id.to_le_bytes()], |r| r.get(0))?;
        Ok(output)
    }

    pub(crate) fn get_hashes(&mut self) -> Result<HashSet<u64>> {
        let map = self
            .conn
            .prepare(GET_HASHES)?
            .query_map([], |row| {
                let hash_bytes: Vec<u8> = row.get("id")?;
                let hash_array: [u8; 8] = hash_bytes
                    .try_into()
                    .expect("Failed to convert hash bytes to array");
                Ok(u64::from_le_bytes(hash_array))
            })?
            .filter_map(Result::ok)
            .collect::<HashSet<u64>>();

        Ok(map)
    }

    // =====================
    //   ARTIST AND ALBUMS
    // =====================

    pub(crate) fn insert_artists(&mut self, artists: &HashSet<&str>) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut insert_artists = tx.prepare(INSERT_ARTIST)?;
            for artist in artists {
                insert_artists.execute(params![artist])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub(crate) fn insert_albums(&mut self, aa_binding: &HashSet<(&str, &str)>) -> Result<()> {
        let artist_map = self.get_artist_map_name_to_id()?;
        let tx = self.conn.transaction()?;
        {
            let mut insert_albums = tx.prepare(INSERT_ALBUM)?;
            for (album_artist, album) in aa_binding {
                let artist_id = artist_map.get(*album_artist);
                insert_albums.execute(params![album, artist_id])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub(crate) fn get_album_map(&mut self) -> Result<Vec<(Arc<String>, Arc<String>)>> {
        let map = self
            .conn
            .prepare(ALBUM_BUILDER)?
            .query_map([], |row| {
                let artist_id = row.get("artist_id")?;
                let album_id = row.get("id")?;

                let artist = match self.artist_map.get(&artist_id) {
                    Some(a) => Arc::clone(&a),
                    None => unreachable!(),
                };
                let album = match self.album_map.get(&album_id) {
                    Some(a) => Arc::clone(&a),
                    None => unreachable!(),
                };

                Ok((album, artist))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(map)
    }

    /// Returns a hashmap of String: i64
    fn get_artist_map_name_to_id(&self) -> Result<HashMap<String, i64>> {
        let artist_map = self
            .conn
            .prepare(GET_ARTIST_MAP)?
            .query_map([], |row| Ok((row.get("name")?, row.get("id")?)))?
            .collect::<Result<HashMap<_, _>, _>>()?;

        Ok(artist_map)
    }

    /// Get album title to ID mapping from a transaction
    fn get_album_map_name_to_id(&self) -> Result<HashMap<(String, i64), i64>> {
        let album_map = self
            .conn
            .prepare(GET_ALBUM_MAP)?
            .query_map([], |row| {
                Ok(((row.get("title")?, row.get("artist_id")?), row.get("id")?))
            })?
            .collect::<Result<HashMap<_, _>, _>>()?;

        Ok(album_map)
    }

    fn set_artist_map(&mut self) -> Result<()> {
        self.artist_map = self
            .conn
            .prepare(GET_ARTIST_MAP)?
            .query_map([], |row| {
                Ok((row.get("id")?, Arc::from(row.get::<_, String>("name")?)))
            })?
            .collect::<Result<HashMap<_, _>, _>>()?;

        Ok(())
    }

    fn set_album_map(&mut self) -> Result<()> {
        self.album_map = self
            .conn
            .prepare(GET_ALBUM_MAP)?
            .query_map([], |row| {
                Ok((row.get("id")?, Arc::from(row.get::<_, String>("title")?)))
            })?
            .collect::<Result<HashMap<_, _>, _>>()?;

        Ok(())
    }

    // =============
    //   WAVEFORMS
    // =============

    pub fn get_waveform(&mut self, id: u64) -> Result<Vec<f32>> {
        let blob: Vec<u8> =
            self.conn
                .query_row(GET_WAVEFORM, params![id.to_le_bytes()], |row| row.get(0))?;
        Ok(bincode::decode_from_slice(&blob, bincode::config::standard())?.0)
    }

    pub fn set_waveform(&mut self, id: u64, wf: &[f32]) -> Result<()> {
        let serialized = bincode::encode_to_vec(wf, bincode::config::standard())?;

        self.conn
            .execute(INSERT_WAVEFORM, params![id.to_le_bytes(), serialized])?;

        Ok(())
    }

    // ============
    //   HISTORY
    // ============

    pub fn save_history_to_db(&mut self, history: &[u64]) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            // Create timestamp
            let timestamp = std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Could not create timestamp!")
                .as_secs() as i64;

            let mut stmt = tx.prepare(INSERT_INTO_HISTORY)?;

            // Since all timestamps are generated as we go into this
            // function, subtract index value from timestamp value to
            // maintain prior ordering
            for (idx, song_id) in history.iter().enumerate() {
                stmt.execute(params![song_id.to_le_bytes(), timestamp - idx as i64])?;
            }
            tx.execute(DELETE_FROM_HISTORY, [])?;
        }
        tx.commit()?;

        Ok(())
    }

    pub fn import_history(
        &mut self,
        song_map: &IndexMap<u64, Arc<SimpleSong>>,
    ) -> Result<VecDeque<Arc<SimpleSong>>> {
        let mut history = VecDeque::new();

        let mut stmt = self.conn.prepare(LOAD_HISTORY)?;
        let rows = stmt.query_map([], |row| {
            let song_id_bytes: Vec<u8> = row.get("song_id")?;
            let song_id_array: [u8; 8] =
                song_id_bytes.try_into().expect("Invalid hash bytes length");
            let song_id = u64::from_le_bytes(song_id_array);
            Ok(song_id)
        })?;

        for row in rows {
            if let Ok(song_id) = row {
                if let Some(song) = song_map.get(&song_id) {
                    history.push_back(Arc::clone(song));
                }
            }
        }

        Ok(history)
    }

    // =================
    //   ROOTS & PATHS
    // =================

    pub(crate) fn get_roots(&mut self) -> Result<HashSet<String>> {
        let roots = self
            .conn
            .prepare(GET_ROOTS)?
            .query_map([], |row| row.get("path"))?
            .collect::<Result<HashSet<String>, _>>()?;

        Ok(roots)
    }

    pub(crate) fn set_root(&mut self, path: &PathBuf) -> Result<()> {
        self.conn.execute(SET_ROOT, params![path.to_str()])?;
        Ok(())
    }

    pub(crate) fn delete_root(&mut self, path: &PathBuf) -> Result<()> {
        self.conn.execute(DELETE_ROOT, params![path.to_str()])?;
        Ok(())
    }
}

// .\src\database\playlists.rs
use indexmap::IndexMap;

use crate::{Database, database::queries::*, domain::Playlist};
use anyhow::Result;
use rusqlite::params;

impl Database {
    pub fn get_playlists(&mut self) -> Result<Vec<Playlist>> {
        let mut stmt = self.conn.prepare_cached(GET_PLAYLISTS)?;

        let rows = stmt.query_map([], |r| {
            let id: i64 = r.get("id")?;
            let name: String = r.get("name")?;

            Ok(Playlist::new(id, name))
        })?;

        let mut playlists = vec![];
        for row in rows {
            if let Ok(playlist) = row {
                playlists.push(playlist);
            }
        }

        Ok(playlists)
    }

    pub fn create_playlist(&mut self, name: &str) -> Result<()> {
        self.conn.execute(CREATE_NEW_PLAYLIST, params![name])?;

        Ok(())
    }

    pub fn delete_playlist(&mut self, id: i64) -> Result<()> {
        self.conn.execute(DELETE_PLAYLIST, params![id])?;

        Ok(())
    }

    pub fn rename_playlist(&mut self, new_name: &str, playlist_id: i64) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            tx.execute(RENAME_PLAYLIST, params![new_name, playlist_id])?;
            tx.execute(UPDATE_PLAYLIST, params![playlist_id])?;
        }
        tx.commit()?;

        Ok(())
    }

    pub fn add_to_playlist(&mut self, song_id: u64, playlist_id: i64) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            ADD_SONG_TO_PLAYLIST,
            params![song_id.to_le_bytes(), playlist_id],
        )?;
        tx.execute(UPDATE_PLAYLIST, params![playlist_id])?;

        tx.commit()?;

        Ok(())
    }

    pub fn add_to_playlist_bulk(&mut self, songs: Vec<u64>, playlist_id: i64) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let start_pos = tx
                .query_row(GET_PLAYLIST_POSITION_NEXT, params![playlist_id], |row| {
                    row.get(0)
                })
                .unwrap_or(0)
                + 1;

            let mut stmt = tx.prepare_cached(ADD_SONG_TO_PLAYLIST_WITH_POSITION)?;

            for (i, song) in songs.iter().enumerate() {
                stmt.execute(params![
                    song.to_le_bytes(),
                    playlist_id,
                    start_pos + i as i64
                ])?;
            }

            tx.execute(UPDATE_PLAYLIST, params![playlist_id])?;
        }
        tx.commit()?;

        Ok(())
    }

    pub fn remove_from_playlist(&mut self, ps_id: &[i64]) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare_cached(REMOVE_SONG_FROM_PLAYLIST)?;
            for id in ps_id {
                stmt.execute(params![id])?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    pub fn swap_position(&mut self, ps_id1: i64, ps_id2: i64, playlist_id: i64) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let pos1: i64 = tx.query_row(GET_PLAYLIST_POS, params![ps_id1], |row| row.get(0))?;

            let pos2: i64 = tx.query_row(GET_PLAYLIST_POS, params![ps_id2], |row| row.get(0))?;

            // Three-step swap to avoid unique constraint violation
            tx.execute(UPDATE_PLAYLIST_POS, params![-1, ps_id1])?;
            tx.execute(UPDATE_PLAYLIST_POS, params![pos1, ps_id2])?;
            tx.execute(UPDATE_PLAYLIST_POS, params![pos2, ps_id1])?;

            tx.execute(UPDATE_PLAYLIST, params![playlist_id])?;
        }

        tx.commit()?;

        Ok(())
    }

    pub fn build_playlists(&mut self) -> Result<IndexMap<(i64, String), Vec<(i64, u64)>>> {
        let mut stmt = self.conn.prepare_cached(PLAYLIST_BUILDER)?;

        let rows = stmt.query_map([], |r| {
            let ps_id: Option<i64> = r.get("id")?;
            let name: String = r.get("name")?;
            let playlist_id: i64 = r.get("playlist_id")?;

            let song_id: Option<u64> = match r.get::<_, Option<Vec<u8>>>("song_id")? {
                Some(hash_bytes) => {
                    let hash_array: [u8; 8] = hash_bytes.try_into().map_err(|_| {
                        rusqlite::Error::InvalidColumnType(
                            2,
                            "song_id".to_string(),
                            rusqlite::types::Type::Blob,
                        )
                    })?;
                    Some(u64::from_le_bytes(hash_array))
                }
                None => None,
            };

            Ok((playlist_id, song_id, ps_id, name))
        })?;

        let mut playlist_map: IndexMap<(i64, String), Vec<(i64, u64)>> = IndexMap::new();

        for row in rows {
            let (playlist_id, song_id_opt, ps_id_opt, name) = row?;

            let entry = playlist_map
                .entry((playlist_id, name))
                .or_insert_with(Vec::new);

            if let (Some(song_id), Some(ps_id)) = (song_id_opt, ps_id_opt) {
                entry.push((ps_id, song_id))
            }
        }

        Ok(playlist_map)
    }
}

// .\src\database\queries.rs
pub const GET_WAVEFORM: &str = "
    SELECT waveform FROM waveforms
    WHERE song_id = ?
";

pub const INSERT_WAVEFORM: &str = "
    INSERT or IGNORE INTO waveforms (song_id, waveform)
    VALUES (?1, ?2)
";

pub const GET_ALL_SONGS: &str = "
    SELECT
        s.id,
        s.path,
        s.title,
        s.year,
        s.track_no,
        s.disc_no,
        s.duration,
        s.artist_id,
        s.album_id,
        s.format,
        a.title as album,
        a.artist_id as album_artist
    from songs s
    INNER JOIN albums a ON a.id = s.album_id
    ORDER BY 
        album ASC, 
        disc_no ASC, 
        track_no ASC
";

pub const INSERT_SONG: &str = "
    INSERT OR REPLACE INTO songs (
        id,
        title, 
        year,
        path, 
        artist_id, 
        album_id, 
        track_no, 
        disc_no, 
        duration, 
        sample_rate, 
        format
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11
)";

pub const INSERT_ARTIST: &str = "
    INSERT OR IGNORE INTO artists (
    name
) VALUES (?1)
";

pub const INSERT_ALBUM: &str = "
    INSERT OR IGNORE INTO albums (
    title,
    artist_id
) VALUES (?1, ?2)
";

pub const GET_PATH: &str = "
    SELECT path FROM songs
    WHERE id = ?
";

pub const GET_ARTIST_MAP: &str = "
    SELECT id, name FROM artists
";

pub const GET_ALBUM_MAP: &str = "
    SELECT id, title, artist_id FROM albums
";

pub const ALBUM_BUILDER: &str = "
    SELECT 
        id, artist_id 
    FROM albums
    ORDER BY title
";

pub const GET_ROOTS: &str = "
    SELECT path FROM roots
";

pub const SET_ROOT: &str = "
    INSERT OR IGNORE INTO roots (path) VALUES (?)
";

pub const DELETE_ROOT: &str = "
    DELETE FROM roots WHERE path = ?
";

pub const GET_HASHES: &str = "
    SELECT id FROM songs
";

pub const DELETE_SONGS: &str = "
    DELETE FROM songs WHERE id = ?
";

pub const LOAD_HISTORY: &str = "
    SELECT song_id FROM history
    ORDER BY timestamp DESC
    LIMIT 50
";

pub const INSERT_INTO_HISTORY: &str = "
    INSERT INTO history (song_id, timestamp) VALUES (?, ?)";

pub const DELETE_FROM_HISTORY: &str = "
    DELETE FROM history WHERE id NOT IN 
        (SELECT id FROM history ORDER BY timestamp DESC LIMIT 50)
";

pub const UPDATE_PLAY_COUNT: &str = "
    INSERT INTO plays 
        (song_id, count)
    VALUES (?1, ?2)
    ON CONFLICT(song_id) DO UPDATE SET
        count = count + ?2 
        WHERE song_id = ?1
";

pub const GET_SESSION_STATE: &str = "
    SELECT value FROM session_state WHERE key = ?
";

pub const GET_UI_SNAPSHOT: &str = "
    SELECT key, value 
        FROM session_state 
        WHERE key LIKE 'ui_%'";

pub const SET_SESSION_STATE: &str = "
    INSERT OR REPLACE INTO session_state (key, value)
        VALUES (?, ?)
";

pub const CREATE_NEW_PLAYLIST: &str = "
    INSERT OR IGNORE INTO playlists (name, updated_at) 
        VALUES (?, strftime('%s', 'now'))
";

pub const UPDATE_PLAYLIST: &str = "
    UPDATE playlists
        SET updated_at = strftime('%s', 'now')
        WHERE id = ?
";

pub const DELETE_PLAYLIST: &str = "
    DELETE FROM playlists
        WHERE id = ?
";

pub const GET_PLAYLIST_POSITION_NEXT: &str = "
    SELECT COALESCE(MAX(position), 0)  
    FROM playlist_songs WHERE playlist_id = ?
";

pub const ADD_SONG_TO_PLAYLIST: &str = "
    INSERT INTO playlist_songs (
        song_id, 
        playlist_id, 
        position)
    VALUES (
        ?1, 
        ?2, 
        COALESCE((SELECT MAX(position) + 1
        FROM playlist_songs WHERE playlist_id = ?2), 1)
    )
";

pub const ADD_SONG_TO_PLAYLIST_WITH_POSITION: &str = "
    INSERT INTO playlist_songs (
        song_id, 
        playlist_id, 
        position
    )
    VALUES (
        ?1, 
        ?2, 
        ?3
    )
    
";

pub const GET_PLAYLISTS: &str = "
    SELECT id, name 
        FROM playlists
        ORDER BY updated_at DESC
";

pub const PLAYLIST_BUILDER: &str = "
    SELECT 
        ps.id,
        ps.song_id, 
        p.id as playlist_id, 
        p.name 
    FROM playlists p
    LEFT JOIN playlist_songs ps 
        ON p.id = ps.playlist_id
    ORDER BY p.updated_at DESC, COALESCE(ps.position, 0) ASC
";

pub const REMOVE_SONG_FROM_PLAYLIST: &str = "
    DELETE FROM playlist_songs
    WHERE id = ?;
";

pub const GET_PLAYLIST_POS: &str = " 
    SELECT position FROM playlist_songs WHERE id = ?
";

pub const UPDATE_PLAYLIST_POS: &str = "
    UPDATE playlist_songs SET position = ? WHERE id = ?
";

pub const RENAME_PLAYLIST: &str = "
    UPDATE playlists SET name = ? WHERE id = ?
";

// .\src\database\snapshot.rs
use anyhow::Result;
use rusqlite::params;

use crate::{
    Database,
    database::queries::{GET_SESSION_STATE, GET_UI_SNAPSHOT, SET_SESSION_STATE},
    ui_state::UiSnapshot,
};

impl Database {
    pub fn get_session_state(&mut self, key: &str) -> Result<Option<String>> {
        match self.conn.query_row(GET_SESSION_STATE, params![key], |row| {
            row.get::<_, String>(0)
        }) {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn save_ui_snapshot(&mut self, snapshot: UiSnapshot) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare(SET_SESSION_STATE)?;
            for (key, value) in snapshot.to_pairs() {
                stmt.execute(params![key, value])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn load_ui_snapshot(&mut self) -> Result<Option<UiSnapshot>> {
        let mut stmt = self.conn.prepare(GET_UI_SNAPSHOT)?;

        let values: Vec<(String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(Result::ok)
            .collect();

        if values.is_empty() {
            Ok(None)
        } else {
            Ok(Some(UiSnapshot::from_values(values)))
        }
    }
}

// .\src\database\tables.rs
pub const CREATE_TABLES: &str = r"
    CREATE TABLE IF NOT EXISTS roots(
        id INTEGER PRIMARY KEY,
        path TEXT UNIQUE NOT NULL
    );

    CREATE TABLE IF NOT EXISTS songs(
        id BLOB PRIMARY KEY,
        title TEXT NOT NULL,
        year INTEGER,
        path TEXT UNIQUE NOT NULL,
        artist_id INTEGER,
        album_id INTEGER,
        track_no INTEGER,
        disc_no INTEGER,
        duration REAL,
        sample_rate INTEGER,
        format INTEGER,
        FOREIGN KEY(artist_id) REFERENCES artists(id),
        FOREIGN KEY(album_id) REFERENCES albums(id)
    );

    CREATE TABLE IF NOT EXISTS artists(
        id INTEGER PRIMARY KEY,
        name TEXT UNIQUE NOT NULL
    );

    CREATE TABLE IF NOT EXISTS albums(
        id INTEGER PRIMARY KEY,
        title TEXT NOT NULL,
        artist_id INTEGER,
        FOREIGN KEY(artist_id) REFERENCES artists(id),
        UNIQUE (title, artist_id)
    );

    CREATE TABLE IF NOT EXISTS waveforms(
        song_id BLOB PRIMARY KEY,
        waveform BLOB,
        FOREIGN KEY(song_id) REFERENCES songs(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS history(
        id INTEGER PRIMARY KEY,
        song_id BLOB NOT NULL,
        timestamp INTEGER NOT NULL,
        FOREIGN KEY(song_id) REFERENCES songs(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS plays(
        song_id BLOB PRIMARY KEY,
        count INTEGER,
        FOREIGN KEY(song_id) REFERENCES songs(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS session_state(
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS playlists(
        id INTEGER PRIMARY KEY,
        name TEXT UNIQUE NOT NULL,
        updated_at INTEGER NOT NULL
    );

    CREATE TABLE IF NOT EXISTS playlist_songs(
        id INTEGER PRIMARY KEY,
        song_id BLOB NOT NULL,
        playlist_id INTEGER NOT NULL,
        position INTEGER NOT NULL,
        FOREIGN KEY (song_id) REFERENCES songs(id) ON DELETE CASCADE,
        FOREIGN KEY (playlist_id) REFERENCES playlists(id) ON DELETE CASCADE,
        UNIQUE(playlist_id, position)
    );
";

// .\src\database\worker.rs
use crate::{database::Database, domain::SimpleSong, ui_state::UiSnapshot};
use anyhow::{Result, anyhow};
use indexmap::IndexMap;
use std::{
    collections::{HashSet, VecDeque},
    sync::{Arc, mpsc},
    thread,
};

pub enum DbMessage {
    Operation(Box<dyn FnOnce(&mut Database) + Send>),
    Shutdown,
}

pub struct DbWorker {
    sender: mpsc::Sender<DbMessage>,
    pub handle: Option<thread::JoinHandle<()>>,
}

impl DbWorker {
    pub fn new() -> Result<Self> {
        let (sender, receiver) = mpsc::channel::<DbMessage>();

        let handle = thread::spawn(move || {
            let mut db = match Database::open() {
                Ok(db) => db,
                Err(e) => {
                    eprintln!("Failed to open database in worker: {}", e);
                    return;
                }
            };

            while let Ok(msg) = receiver.recv() {
                match msg {
                    DbMessage::Operation(operation) => {
                        operation(&mut db);
                    }
                    DbMessage::Shutdown => {
                        break;
                    }
                }
            }
        });

        Ok(DbWorker {
            sender,
            handle: Some(handle),
        })
    }

    // Fire and forget operation
    pub fn execute<F>(&self, operation: F)
    where
        F: FnOnce(&mut Database) + Send + 'static,
    {
        let _ = self.sender.send(DbMessage::Operation(Box::new(operation)));
    }

    // Operations that need a response
    pub fn execute_sync<F, T>(&self, operation: F) -> Result<T>
    where
        F: FnOnce(&mut Database) -> Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let (result_tx, result_rx) = mpsc::channel();

        self.execute(move |db| {
            let result = operation(db);
            let _ = result_tx.send(result);
        });

        result_rx
            .recv()
            .map_err(|_| anyhow::anyhow!("Worker dropped"))?
    }

    pub fn shutdown(&mut self) -> Result<()> {
        self.sender
            .send(DbMessage::Shutdown)
            .expect("Could not shutdown dbworker");

        if let Some(handle) = self.handle.take() {
            handle
                .join()
                .map_err(|_| anyhow!("Worker thread panicked!"))?;
        }

        Ok(())
    }
}

// Convenience functions
impl DbWorker {
    pub fn create_playlist(&self, name: String) -> Result<()> {
        self.execute_sync(move |db| db.create_playlist(&name))
    }

    pub fn delete_playlist(&self, id: i64) -> Result<()> {
        self.execute_sync(move |db| db.delete_playlist(id))
    }

    pub fn add_to_playlist(&self, song_id: u64, playlist_id: i64) -> Result<()> {
        self.execute_sync(move |db| db.add_to_playlist(song_id, playlist_id))
    }

    pub fn add_to_playlist_bulk(&self, song_ids: Vec<u64>, playlist_id: i64) -> Result<()> {
        self.execute_sync(move |db| db.add_to_playlist_bulk(song_ids, playlist_id))
    }

    pub fn rename_playlist(&self, id: i64, new_name: String) -> Result<()> {
        self.execute_sync(move |db| db.rename_playlist(&new_name, id))
    }

    pub fn remove_from_playlist(&self, ps_ids: Vec<i64>) -> Result<()> {
        self.execute_sync(move |db| db.remove_from_playlist(&ps_ids))
    }

    pub fn swap_position(&self, ps_id1: i64, ps_id2: i64, playlist_id: i64) -> Result<()> {
        self.execute_sync(move |db| db.swap_position(ps_id1, ps_id2, playlist_id))
    }

    pub fn get_hashes(&self) -> Result<HashSet<u64>> {
        self.execute_sync(move |db| db.get_hashes())
    }

    pub fn build_playlists(&mut self) -> Result<IndexMap<(i64, String), Vec<(i64, u64)>>> {
        self.execute_sync(move |db| db.build_playlists())
    }

    pub fn save_history(&self, history: Vec<u64>) -> Result<()> {
        self.execute_sync(move |db| db.save_history_to_db(&history))
    }

    pub fn save_ui_snapshot(&self, snapshot: UiSnapshot) -> Result<()> {
        self.execute_sync(move |db| db.save_ui_snapshot(snapshot))
    }

    pub fn load_ui_snapshot(&self) -> Result<Option<UiSnapshot>> {
        self.execute_sync(move |db| db.load_ui_snapshot())
    }

    pub fn get_all_songs(&self) -> Result<IndexMap<u64, Arc<SimpleSong>>> {
        self.execute_sync(move |db| db.get_all_songs())
    }

    pub fn import_history(
        &self,
        song_map: IndexMap<u64, Arc<SimpleSong>>,
    ) -> Result<VecDeque<Arc<SimpleSong>>> {
        self.execute_sync(move |db| db.import_history(&song_map))
    }

    pub fn save_history_to_db(&self, history: Vec<u64>) -> Result<()> {
        self.execute_sync(move |db| db.save_history_to_db(&history))
    }

    pub fn get_song_path(&self, id: u64) -> Result<String> {
        self.execute_sync(move |db| db.get_song_path(id))
    }

    // Fire-and-forget operations
    pub fn update_play_count(&self, song_id: u64) {
        self.execute(move |db| {
            let _ = db.update_play_count(song_id);
        });
    }

    pub fn set_waveform(&self, song_id: u64, waveform: Vec<f32>) {
        self.execute(move |db| {
            let _ = db.set_waveform(song_id, &waveform);
        });
    }
}

impl Drop for DbWorker {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

// .\src\domain\album.rs
use super::SimpleSong;
use std::sync::Arc;

#[derive(Default, Clone)]
pub struct Album {
    pub title: Arc<String>,
    pub artist: Arc<String>,
    pub year: Option<u32>,
    pub tracklist: Vec<Arc<SimpleSong>>,
}

impl Album {
    pub fn from_aa(title: &Arc<String>, artist: &Arc<String>) -> Self {
        Album {
            title: Arc::clone(&title),
            artist: Arc::clone(&artist),
            year: None,
            tracklist: Vec::new(),
        }
    }

    pub fn get_tracklist(&self) -> Vec<Arc<SimpleSong>> {
        self.tracklist.clone()
    }
}

// .\src\domain\filetype.rs
use rusqlite::{
    types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, Value, ValueRef},
    Result as RusqliteResult, ToSql,
};
use std::fmt::Display;

#[allow(clippy::upper_case_acronyms)]
#[derive(Default, Eq, PartialEq, Copy, Clone, Hash)]
pub enum FileType {
    MP3 = 1,
    M4A = 2,
    OGG = 3,
    WAV = 4,
    FLAC = 5,
    #[default]
    ERR = 0,
}

impl From<&str> for FileType {
    fn from(str: &str) -> Self {
        match str {
            "mp3" => Self::MP3,
            "m4a" => Self::M4A,
            "ogg" => Self::OGG,
            "flac" => Self::FLAC,
            "wav" => Self::WAV,
            _ => Self::ERR,
        }
    }
}

impl FromSql for FileType {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value {
            ValueRef::Integer(i) => Ok(FileType::from_i64(i)),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

impl ToSql for FileType {
    fn to_sql(&self) -> RusqliteResult<rusqlite::types::ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Integer(self.to_i64())))
    }
}

impl Display for FileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            FileType::MP3 => write!(f, "ᵐᵖ³"),
            FileType::M4A => write!(f, "ᵐ⁴ᵃ"),
            FileType::OGG => write!(f, "ᵒᵍᵍ"),
            FileType::WAV => write!(f, "ʷᵃᵛ"),
            FileType::FLAC => write!(f, "ᶠˡᵃᶜ"),
            FileType::ERR => write!(f, "ERR"),
        }
    }
}

impl FileType {
    pub fn from_i64(value: i64) -> Self {
        match value {
            1 => Self::MP3,
            2 => Self::M4A,
            3 => Self::OGG,
            4 => Self::WAV,
            5 => Self::FLAC,
            _ => Self::ERR,
        }
    }

    pub fn to_i64(&self) -> i64 {
        *self as i64
    }
}

// .\src\domain\long_song.rs
use super::{FileType, SongInfo};
use crate::{calculate_signature, database::Database, get_readable_duration};
use anyhow::{anyhow, Context, Result};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use symphonia::core::{
    io::MediaSourceStream,
    meta::{StandardTagKey, Value},
    probe::Hint,
};

#[derive(Default)]
pub struct LongSong {
    pub(crate) id: u64,
    pub(crate) title: String,
    pub(crate) artist: Arc<String>,
    pub(crate) album_artist: Arc<String>,
    pub(crate) album: Arc<String>,
    pub(crate) track_no: Option<u32>,
    pub(crate) disc_no: Option<u32>,
    pub(crate) duration: Duration,
    pub(crate) sample_rate: u32,
    pub(crate) year: Option<u32>,
    pub(crate) filetype: FileType,
    pub(crate) path: PathBuf,
}

impl LongSong {
    pub fn new(path: PathBuf) -> Self {
        LongSong {
            path,
            ..Default::default()
        }
    }

    pub fn build_song_symphonia<P: AsRef<Path>>(path_raw: P) -> Result<LongSong> {
        let path = path_raw.as_ref();

        let extension = path.extension();
        let format = match extension {
            Some(n) => FileType::from(n.to_str().unwrap()),
            None => return Err(anyhow!("Unsuppored extension: {:?}", path.extension())),
        };

        let src = std::fs::File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(src), Default::default());
        let mut hint = Hint::new();

        if let Some(ext) = extension {
            if let Some(ext_str) = ext.to_str() {
                hint.with_extension(ext_str);
            }
        }

        let mut probed = symphonia::default::get_probe().format(
            &hint,
            mss,
            &Default::default(),
            &Default::default(),
        )?;

        let mut song_info = LongSong::new(PathBuf::from(path));

        song_info.filetype = format;
        song_info.id = calculate_signature(path)?;

        let track = probed.format.default_track().context("No default track")?;

        if let Some(n_frames) = track.codec_params.n_frames {
            let sample_rate = track
                .codec_params
                .sample_rate
                .context("Sample rate is not specified")?;

            let duration_raw = Duration::from_secs_f32(n_frames as f32 / sample_rate as f32);

            song_info.sample_rate = sample_rate;
            song_info.duration = duration_raw;
        }

        let metadata = match probed.metadata.get() {
            Some(m) => m,
            None => probed.format.metadata(),
        };

        let tags = metadata
            .current()
            .context("Could not get current metadata from file!")?
            .tags();

        for tag in tags {
            if let Some(key) = tag.std_key {
                song_info.match_tags(key, &tag.value);
            }
        }

        if !song_info.artist.is_empty() && song_info.album_artist.is_empty() {
            song_info.album_artist = Arc::clone(&song_info.artist);
        }

        if song_info.title.is_empty() {
            song_info.title = path
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default()
        }

        if song_info.filetype == FileType::M4A {
            let tag = mp4ameta::Tag::read_from_path(path).unwrap();
            song_info.disc_no = tag.disc_number().map(u32::from);
        }

        Ok(song_info)
    }

    fn match_tags(&mut self, key: StandardTagKey, value: &Value) {
        match key {
            StandardTagKey::TrackTitle => self.title = value.to_string(),
            StandardTagKey::Album => self.album = Arc::new(value.to_string()),
            StandardTagKey::Artist => self.artist = Arc::new(value.to_string()),
            StandardTagKey::AlbumArtist => self.album_artist = Arc::new(value.to_string()),
            StandardTagKey::Date => {
                self.year = value
                    .to_string()
                    .split_once('-')
                    .map(|(year, _)| year)
                    .unwrap_or(&value.to_string())
                    .parse::<u32>()
                    .ok()
            }
            StandardTagKey::TrackNumber => {
                self.track_no = value
                    .to_string()
                    .split_once('/')
                    .map(|(num, _)| num)
                    .unwrap_or(&value.to_string())
                    .parse::<u32>()
                    .ok()
            }
            StandardTagKey::DiscNumber => {
                self.disc_no = value
                    .to_string()
                    .split_once('/')
                    .map(|(num, _)| num)
                    .unwrap_or(&value.to_string())
                    .parse::<u32>()
                    .ok()
            }
            _ => {}
        }
    }

    pub fn get_path(&self, db: &mut Database) -> Result<String> {
        db.get_song_path(self.id)
    }
}

impl SongInfo for LongSong {
    fn get_id(&self) -> u64 {
        self.id
    }

    fn get_title(&self) -> &str {
        &self.title
    }

    fn get_artist(&self) -> &str {
        &self.artist
    }

    fn get_album(&self) -> &str {
        &self.album
    }

    fn get_duration(&self) -> Duration {
        self.duration
    }

    fn get_duration_f32(&self) -> f32 {
        self.duration.as_secs_f32()
    }

    fn get_duration_str(&self) -> String {
        get_readable_duration(self.duration, crate::DurationStyle::Compact)
    }
}

// .\src\domain\mod.rs
mod album;
mod filetype;
mod long_song;
mod playlist;
mod queue_song;
mod simple_song;
mod waveform;

pub use album::Album;
pub use filetype::FileType;
pub use long_song::LongSong;
pub use playlist::Playlist;
pub use playlist::PlaylistSong;
pub use queue_song::QueueSong;
pub use simple_song::SimpleSong;

pub use waveform::generate_waveform;

pub trait SongInfo {
    fn get_id(&self) -> u64;
    fn get_title(&self) -> &str;
    fn get_artist(&self) -> &str;
    fn get_album(&self) -> &str;
    fn get_duration(&self) -> std::time::Duration;
    fn get_duration_f32(&self) -> f32;
    fn get_duration_str(&self) -> String;
}

pub trait SongDatabase {
    fn get_path(&self) -> anyhow::Result<String>;
    fn update_play_count(&self) -> anyhow::Result<()>;
    fn get_waveform(&self) -> anyhow::Result<Vec<f32>>;
    fn set_waveform(&self, wf: &[f32]) -> anyhow::Result<()>;
}

// .\src\domain\playlist.rs
use super::SimpleSong;
use std::sync::Arc;

pub struct Playlist {
    pub id: i64,
    pub name: String,
    pub tracklist: Vec<PlaylistSong>,
}

impl Playlist {
    pub fn new(id: i64, name: String) -> Self {
        Playlist {
            id,
            name,
            tracklist: Vec::new(),
        }
    }

    pub fn get_tracks(&self) -> Vec<Arc<SimpleSong>> {
        self.tracklist
            .iter()
            .map(|s| Arc::clone(&s.song))
            .collect::<Vec<_>>()
    }
}

pub struct PlaylistSong {
    pub id: i64,
    pub song: Arc<SimpleSong>,
}

// .\src\domain\queue_song.rs
use super::{SimpleSong, SongInfo};
use crate::{Database, domain::SongDatabase, get_readable_duration};
use anyhow::Result;
use std::{sync::Arc, time::Duration};

pub struct QueueSong {
    pub meta: Arc<SimpleSong>,
    pub path: String,
}

impl SongInfo for QueueSong {
    fn get_id(&self) -> u64 {
        self.meta.id
    }

    fn get_title(&self) -> &str {
        &self.meta.title
    }

    fn get_artist(&self) -> &str {
        &self.meta.artist
    }

    fn get_album(&self) -> &str {
        &self.meta.album
    }

    fn get_duration(&self) -> Duration {
        self.meta.duration
    }

    fn get_duration_f32(&self) -> f32 {
        self.meta.duration.as_secs_f32()
    }

    fn get_duration_str(&self) -> String {
        get_readable_duration(self.meta.duration, crate::DurationStyle::Compact)
    }
}

impl SongDatabase for QueueSong {
    /// Returns the path of a song as a String
    fn get_path(&self) -> Result<String> {
        let mut db = Database::open()?;
        db.get_song_path(self.meta.id)
    }

    /// Update the play_count of the song
    fn update_play_count(&self) -> Result<()> {
        let mut db = Database::open()?;
        db.update_play_count(self.meta.id)
    }

    /// Retrieve the waveform of a song
    /// returns Result<Vec<f32>>
    fn get_waveform(&self) -> Result<Vec<f32>> {
        let mut db = Database::open()?;
        db.get_waveform(self.meta.id)
    }

    /// Store the waveform of a song in the databse
    fn set_waveform(&self, wf: &[f32]) -> Result<()> {
        let mut db = Database::open()?;
        db.set_waveform(self.meta.id, wf)
    }
}

// .\src\domain\simple_song.rs
use super::{FileType, SongInfo};
use crate::{Database, domain::SongDatabase, get_readable_duration};
use anyhow::Result;
use std::{sync::Arc, time::Duration};

#[derive(Default, Hash, Eq, PartialEq)]
pub struct SimpleSong {
    pub(crate) id: u64,
    pub(crate) title: String,
    pub(crate) artist: Arc<String>,
    pub(crate) year: Option<u32>,
    pub(crate) album: Arc<String>,
    pub(crate) album_artist: Arc<String>,
    pub(crate) track_no: Option<u32>,
    pub(crate) disc_no: Option<u32>,
    pub(crate) duration: Duration,
    pub(crate) filetype: FileType,
}

/// DATABASE RELATED METHODS

impl SongDatabase for SimpleSong {
    /// Returns the path of a song as a String
    fn get_path(&self) -> Result<String> {
        let mut db = Database::open()?;
        db.get_song_path(self.id)
    }

    /// Update the play_count of the song
    fn update_play_count(&self) -> Result<()> {
        let mut db = Database::open()?;
        db.update_play_count(self.id)
    }

    /// Retrieve the waveform of a song
    /// returns Result<Vec<f32>>
    fn get_waveform(&self) -> Result<Vec<f32>> {
        let mut db = Database::open()?;
        db.get_waveform(self.id)
    }

    /// Store the waveform of a song in the databse
    fn set_waveform(&self, wf: &[f32]) -> Result<()> {
        let mut db = Database::open()?;
        db.set_waveform(self.id, wf)
    }
}

/// Generic getter methods
impl SongInfo for SimpleSong {
    fn get_id(&self) -> u64 {
        self.id
    }

    fn get_title(&self) -> &str {
        &self.title
    }

    fn get_artist(&self) -> &str {
        &self.artist
    }

    fn get_album(&self) -> &str {
        &self.album
    }

    fn get_duration(&self) -> Duration {
        self.duration
    }

    fn get_duration_f32(&self) -> f32 {
        self.duration.as_secs_f32()
    }

    fn get_duration_str(&self) -> String {
        get_readable_duration(self.duration, crate::DurationStyle::Compact)
    }
}

impl SongInfo for Arc<SimpleSong> {
    fn get_id(&self) -> u64 {
        self.as_ref().get_id()
    }

    fn get_title(&self) -> &str {
        self.as_ref().get_title()
    }

    fn get_artist(&self) -> &str {
        self.as_ref().get_artist()
    }

    fn get_album(&self) -> &str {
        self.as_ref().get_album()
    }

    fn get_duration(&self) -> Duration {
        self.as_ref().get_duration()
    }

    fn get_duration_f32(&self) -> f32 {
        self.as_ref().get_duration_f32()
    }

    fn get_duration_str(&self) -> String {
        self.as_ref().get_duration_str()
    }
}

// .\src\domain\waveform.rs
use anyhow::{Context, Result, anyhow};
use byteorder::{LittleEndian, ReadBytesExt};
use std::{io::Cursor, path::Path, process::Command, time::Duration};

const WF_LEN: usize = 500;
const MIN_SAMPLES_PER_POINT: usize = 200; // Minimum for short files
const MAX_SAMPLES_PER_POINT: usize = 5000; // Maximum for very long files
const SMOOTHING_FACTOR: f32 = 0.2;

/// Generate a waveform using ffmpeg by piping output directly to memory
pub fn generate_waveform<P: AsRef<Path>>(audio_path: P) -> Vec<f32> {
    let path = audio_path.as_ref();

    // TODO: Handle bad waveform data
    match extract_waveform_data(path) {
        Ok(waveform) => waveform,
        Err(_) => {
            vec![0.3; WF_LEN] // Return a flat line if all fails
        }
    }
}

/// Extract duration from audio file using ffmpeg
fn get_audio_duration<P: AsRef<Path>>(audio_path: P) -> Result<Duration> {
    let audio_path_str = audio_path
        .as_ref()
        .to_str()
        .ok_or_else(|| anyhow!("Audio path contains invalid Unicode"))?;

    // Use ffprobe to get duration
    let output = Command::new("ffprobe")
        .args(&[
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            audio_path_str,
        ])
        .output()
        .context("Failed to execute ffprobe")?;

    if !output.status.success() {
        return Err(anyhow!(
            "ffprobe failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let duration_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let duration_secs = duration_str
        .parse::<f64>()
        .context("Failed to parse duration")?;

    Ok(Duration::from_secs_f64(duration_secs))
}

/// Extract waveform data from audio file
fn extract_waveform_data<P: AsRef<Path>>(audio_path: P) -> Result<Vec<f32>> {
    // Get audio duration to calculate optimal sampling
    let duration = match get_audio_duration(&audio_path) {
        Ok(d) => d,
        Err(_) => {
            return Err(anyhow!("Could not determine audio length"));
        }
    };

    // Calculate adaptive samples per point based on duration
    let samples_per_point = calculate_adaptive_samples(duration);

    // Get the path as string, with better error handling
    let audio_path_str = audio_path
        .as_ref()
        .to_str()
        .ok_or_else(|| anyhow!("Audio path contains invalid Unicode"))?;

    // Create a process to pipe audio data directly to memory using ffmpeg
    let mut cmd = Command::new("ffmpeg");
    let output = cmd
        .args(&[
            "-i",
            audio_path_str,
            "-ac",
            "1", // Convert to mono
            "-ar",
            "44100",
            "-af",
            "highpass=f=300,volume=2,treble=gain=3", // Extreme filtering for visual effect
            "-loglevel",
            "warning",
            "-f",
            "f32le",
            "-",
        ])
        .output()
        .context("Failed to execute ffmpeg. Is it installed and in your PATH?")?;

    // Check for errors
    if !output.status.success() {
        return Err(anyhow!(
            "FFmpeg conversion failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // Process the PCM data to generate waveform
    let pcm_data = output.stdout;
    let mut waveform = process_pcm_to_waveform(&pcm_data, samples_per_point)?;

    smooth_waveform(&mut waveform);
    // Normalize the waveform
    normalize_waveform(&mut waveform);

    Ok(waveform)
}

/// Calculate adaptive samples per point based on duration
fn calculate_adaptive_samples(duration: Duration) -> usize {
    let duration_secs = duration.as_secs_f32();
    let sample_rate = 44100.0; // Standard sample rate

    // Calculate total samples in the file
    let total_samples = (duration_secs * sample_rate) as usize;

    // Calculate base samples per point
    // This ensures we consider at least ~10% of the audio total
    let ideal_samples = total_samples / (WF_LEN * 10);

    // Clamp between min and max values
    ideal_samples.clamp(MIN_SAMPLES_PER_POINT, MAX_SAMPLES_PER_POINT)
}

/// Process raw PCM float data into a waveform
fn process_pcm_to_waveform(pcm_data: &[u8], samples_per_point: usize) -> Result<Vec<f32>> {
    // Create a cursor to read the PCM data as 32-bit floats
    let mut cursor = Cursor::new(pcm_data);

    // Calculate total samples and step size
    let total_samples = pcm_data.len() / 4; // Each float is 4 bytes

    // If the file is very short, we might need to adapt our approach
    if total_samples < WF_LEN * samples_per_point {
        return process_short_pcm(pcm_data);
    }

    let sample_step = total_samples / WF_LEN;
    let mut waveform = Vec::with_capacity(WF_LEN);

    for i in 0..WF_LEN {
        let position = i * sample_step * 4; // 4 bytes per float
        if position >= pcm_data.len() {
            break;
        }

        cursor.set_position(position as u64);
        let mut sum_squares = 0.0;
        let mut samples_read = 0;
        let mut max_value = 0.0f32;

        // Read samples_per_point samples or up to the next point
        let max_samples = samples_per_point.min(sample_step);
        for _ in 0..max_samples {
            if cursor.position() >= pcm_data.len() as u64 {
                break;
            }

            match cursor.read_f32::<LittleEndian>() {
                Ok(sample) => {
                    // Track maximum absolute value
                    let abs_sample = sample.abs();
                    if abs_sample > max_value {
                        max_value = abs_sample;
                    }

                    // Sum squares for RMS calculation
                    sum_squares += sample * sample;
                    samples_read += 1;
                }
                Err(_) => break,
            }
        }

        if samples_read > 0 {
            // Use a combination of RMS and peak for better representation
            let rms = (sum_squares / samples_read as f32).sqrt();

            // FIXME: let value = (rms * 0.8 + max_value * 0.2).min(1.0);
            let value = rms.min(1.0);
            waveform.push(value);
        } else {
            waveform.push(0.0);
        }
    }

    // Ensure we have exactly WF_LEN points
    while waveform.len() < WF_LEN {
        waveform.push(0.0);
    }

    Ok(waveform)
}

/// Process very short PCM files
fn process_short_pcm(pcm_data: &[u8]) -> Result<Vec<f32>> {
    let mut cursor = Cursor::new(pcm_data);
    let total_samples = pcm_data.len() / 4;

    // For very short files, we'll divide the available samples evenly
    let samples_per_section = total_samples / WF_LEN.max(1);
    let extra_samples = total_samples % WF_LEN;

    let mut waveform = Vec::with_capacity(WF_LEN);
    let mut position = 0;

    for i in 0..WF_LEN {
        // Calculate how many samples this section should have
        let samples_this_section = if i < extra_samples {
            samples_per_section + 1
        } else {
            samples_per_section
        };

        if samples_this_section == 0 {
            waveform.push(0.0);
            continue;
        }

        cursor.set_position((position * 4) as u64);

        let mut sum_squares = 0.0;
        let mut max_value = 0.0f32;
        let mut samples_read = 0;

        for _ in 0..samples_this_section {
            if cursor.position() >= pcm_data.len() as u64 {
                break;
            }

            match cursor.read_f32::<LittleEndian>() {
                Ok(sample) => {
                    let abs_sample = sample.abs();
                    if abs_sample > max_value {
                        max_value = abs_sample;
                    }
                    sum_squares += sample * sample;
                    samples_read += 1;
                }
                Err(_) => break,
            }
        }

        position += samples_this_section;

        if samples_read > 0 {
            let rms = (sum_squares / samples_read as f32).sqrt();
            //FIXME:  let value = (rms * 0.8 + max_value * 0.2).min(1.0);
            let value = rms.min(1.0);
            waveform.push(value);
        } else {
            waveform.push(0.0);
        }
    }

    while waveform.len() < WF_LEN {
        waveform.push(0.0);
    }

    Ok(waveform)
}

/// Apply a smoothing filter to the waveform with float smoothing factor
fn smooth_waveform(waveform: &mut Vec<f32>) {
    let smoothing_factor = SMOOTHING_FACTOR;
    if waveform.len() <= (smoothing_factor.ceil() as usize * 2 + 1) {
        return; // Not enough points to smooth
    }

    let original = waveform.clone();
    let range = smoothing_factor.ceil() as isize;

    for i in 0..waveform.len() {
        let mut sum = 0.0;
        let mut total_weight = 0.0;

        // Calculate weighted average of surrounding points
        for offset in -range..=range {
            let idx = i as isize + offset;
            if idx >= 0 && idx < original.len() as isize {
                // Weight calculation - based on distance and the smoothing factor
                // Points beyond the float smoothing factor get reduced weight
                let distance = offset.abs() as f32;
                let weight = if distance <= smoothing_factor {
                    // Full weight for points within the smooth factor
                    1.0
                } else {
                    // Partial weight for the fractional part
                    1.0 - (distance - smoothing_factor)
                };

                if weight > 0.0 {
                    sum += original[idx as usize] * weight;
                    total_weight += weight;
                }
            }
        }

        if total_weight > 0.0 {
            waveform[i] = sum / total_weight;
        }
    }
}

/// Normalize the waveform to a 0.0-1.0 range with improved dynamics
fn normalize_waveform(waveform: &mut [f32]) {
    if waveform.is_empty() {
        return;
    }

    // Find min and max using exact values (no percentile)
    let min = *waveform
        .iter()
        .min_by(|a, b| a.total_cmp(b))
        .unwrap_or(&0.0);

    let max = *waveform
        .iter()
        .max_by(|a, b| a.total_cmp(b))
        .unwrap_or(&1.0);

    // Simple normalization like in the old code
    if (max - min).abs() < f32::EPSILON {
        for value in waveform.iter_mut() {
            *value = 0.3;
        }
    } else {
        for value in waveform.iter_mut() {
            *value = (*value - min) / (max - min);
        }
    }
}

// .\src\key_handler\action.rs
use crate::{
    REFRESH_RATE,
    app_core::Concertus,
    key_handler::*,
    ui_state::{LibraryView, Mode, Pane, PlaylistAction, PopupType, SettingsMode, UiState},
};
use anyhow::Result;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent};
use std::time::Duration;

use KeyCode::*;

// #[rustfmt::skip]
pub fn handle_key_event(key_event: KeyEvent, state: &UiState) -> Option<Action> {
    if let Some(action) = global_commands(&key_event, &state) {
        return Some(action);
    }

    match state.get_input_context() {
        InputContext::Popup(popup) => handle_popup(&key_event, &popup),
        InputContext::TrackList(_) => handle_tracklist(&key_event, &state),
        InputContext::AlbumView => handle_album_browser(&key_event),
        InputContext::PlaylistView => handle_playlist_browswer(&key_event),
        InputContext::Search => handle_search_pane(&key_event),
        _ => None,
    }
}

fn global_commands(key: &KeyEvent, state: &UiState) -> Option<Action> {
    let in_search = state.get_pane() == Pane::Search;
    let popup_active = state.popup.is_open();

    // Works on every pane, even search
    match (key.modifiers, key.code) {
        (C, Char('c')) => Some(Action::QUIT),

        (X, Esc) => Some(Action::SoftReset),
        (C, Char(' ')) => Some(Action::TogglePause),

        (C, Char('n')) => Some(Action::PlayNext),
        (C, Char('p')) => Some(Action::PlayPrev),

        (C, Char('m')) => Some(Action::ChangeMode(Mode::Library(LibraryView::Albums))),
        (C, Char('t')) => Some(Action::ChangeMode(Mode::Library(LibraryView::Playlists))),
        (C, Char('q')) => Some(Action::ChangeMode(Mode::Queue)),
        (C, Char('z')) => Some(Action::ChangeMode(Mode::Power)),

        // Works on everything except search or popup
        _ if (!in_search && !popup_active) => match (key.modifiers, key.code) {
            // PLAYBACK COMMANDS
            (X, Char('`')) => Some(Action::ViewSettings),
            (X, Char(' ')) => Some(Action::TogglePause),
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
        (X, Char('v')) => Some(Action::BulkSelect),
        (C, Char('v')) => Some(Action::ClearBulkSelect),

        (X, Left) | (X, Char('h')) => Some(Action::ChangeMode(Mode::Library(
            state.display_state.sidebar_view,
        ))),
        (X, Tab) => Some(Action::ToggleSideBar),
        _ => None,
    };

    if base_action.is_some() {
        return base_action;
    }

    match state.get_mode() {
        Mode::Library(_) => match (key.modifiers, key.code) {
            (S, Char('K')) => Some(Action::ShiftPosition(MoveDirection::Up)),
            (S, Char('J')) => Some(Action::ShiftPosition(MoveDirection::Down)),

            (S, Char('Q')) => Some(Action::QueueEntity),
            (S, Char('V')) => Some(Action::BulkSelectALL),
            (X, Char('x')) => Some(Action::RemoveSong),
            _ => None,
        },

        Mode::Queue => match (key.modifiers, key.code) {
            (X, Char('x')) => Some(Action::RemoveSong),
            (S, Char('K')) => Some(Action::ShiftPosition(MoveDirection::Up)),
            (S, Char('J')) => Some(Action::ShiftPosition(MoveDirection::Down)),
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
        (X, Tab) => Some(Action::ToggleSideBar),
        (X, Char('q')) => Some(Action::QueueEntity),
        (X, Enter) | (X, Right) | (X, Char('l')) => Some(Action::ChangePane(Pane::TrackList)),

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
        (X, Tab) => Some(Action::ToggleSideBar),
        (X, Char('q')) => Some(Action::QueueEntity),

        (X, Enter) | (X, Right) | (X, Char('l')) => Some(Action::ChangePane(Pane::TrackList)),

        (X, Char('c')) => Some(Action::CreatePlaylist),
        (C, Char('d')) => Some(Action::DeletePlaylist),
        _ => None,
    }
}

fn handle_search_pane(key: &KeyEvent) -> Option<Action> {
    match (key.modifiers, key.code) {
        (X, Tab) | (X, Enter) => Some(Action::SendSearch),

        (C, Char('a')) => Some(Action::ChangeMode(Mode::Library(LibraryView::Albums))),

        (_, Left) | (C, Char('h')) => Some(Action::SortColumnsPrev),
        (_, Right) | (C, Char('l')) => Some(Action::SortColumnsNext),
        (_, Char(x)) if ILLEGAL_CHARS.contains(&x) => None,

        _ => Some(Action::UpdateSearch(*key)),
    }
}

fn handle_popup(key: &KeyEvent, popup: &PopupType) -> Option<Action> {
    match popup {
        PopupType::Settings(s) => root_manager(key, s),
        PopupType::Playlist(p) => handle_playlist(key, p),
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
            _ => None,
        },
        Rename => match key.code {
            Esc => Some(Action::ClosePopup),
            Enter => Some(Action::RenamePlaylistConfirm),
            _ => Some(Action::PopupInput(*key)),
        },
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
            Action::CreatePlaylist  => self.ui.create_playlist_popup(),
            Action::CreatePlaylistConfirm => self.ui.create_playlist()?,

            Action::RenamePlaylist  => self.ui.rename_playlist_popup(),
            Action::RenamePlaylistConfirm => self.ui.rename_playlist()?,

            Action::DeletePlaylist  => self.ui.delete_playlist_popup(),
            Action::DeletePlaylistConfirm => self.ui.delete_playlist()?,

            // Queue
            Action::QueueSong       => self.ui.queue_song(None)?,
            Action::QueueEntity     => self.ui.add_to_queue_bulk()?,
            Action::RemoveSong      => self.ui.remove_song()?,
            Action::AddToPlaylist   => self.ui.add_to_playlist_popup(),
            Action::AddToPlaylistConfirm => self.ui.add_to_playlist()?,

            Action::BulkSelect      => self.ui.add_to_bulk_select()?,
            Action::BulkSelectALL   => self.ui.bulk_select_all()?,
            Action::ClearBulkSelect => self.ui.clear_bulk_sel(),

            Action::ShiftPosition(direction) => self.ui.shift_position(direction)?,

            // Ops
            Action::PopupInput(key) => self.ui.process_popup_input(&key),
            Action::ClosePopup      => self.ui.close_popup(),
            Action::SoftReset       => self.ui.soft_reset(),
            Action::UpdateLibrary   => self.update_library()?,
            Action::QUIT            => self.ui.set_mode(Mode::QUIT),

            Action::ViewSettings    => self.activate_settings(),
            Action::PopupScrollUp   => self.popup_scroll_up(),
            Action::PopupScrollDown => self.popup_scroll_down(),
            Action::RootAdd         => self.settings_add_root(),
            Action::RootRemove      => self.settings_remove_root(),
            Action::RootConfirm     => self.settings_root_confirm()?,

            _ => (),
        }
        Ok(())
    }
}

// .\src\key_handler\mod.rs
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

    RenamePlaylist,
    RenamePlaylistConfirm,

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

// .\src\lib.rs
use anyhow::{Result, anyhow};
use ratatui::crossterm::{
    ExecutableCommand,
    cursor::MoveToColumn,
    style::Print,
    terminal::{Clear, ClearType},
};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, UNIX_EPOCH},
};
use ui_state::UiState;
use xxhash_rust::xxh3::xxh3_64;

pub mod app_core;
pub mod database;
pub mod domain;
pub mod key_handler;
pub mod library;
pub mod player;
pub mod tui;
pub mod ui_state;

pub use database::Database;
pub use library::Library;
pub use player::Player;

// ~30fps
pub const REFRESH_RATE: u64 = 33;

/// Create a hash based on...
///  - date of last modification (millis)
///  - file size (bytes)
///  - path as str as bytes
pub fn calculate_signature<P: AsRef<Path>>(path: P) -> anyhow::Result<u64> {
    let metadata = fs::metadata(&path)?;

    let last_mod = metadata.modified()?.duration_since(UNIX_EPOCH)?.as_millis() as i64;
    let size = metadata.len();

    let mut data = Vec::with_capacity(path.as_ref().as_os_str().len() + 16);

    data.extend_from_slice(path.as_ref().as_os_str().as_encoded_bytes());
    data.extend_from_slice(&last_mod.to_le_bytes());
    data.extend_from_slice(&size.to_le_bytes());

    Ok(xxh3_64(&data))
}

pub enum DurationStyle {
    Clean,
    CleanMillis,
    Compact,
    CompactMillis,
}

pub fn get_readable_duration(duration: Duration, style: DurationStyle) -> String {
    let mut secs = duration.as_secs();
    let millis = duration.subsec_millis() % 100;
    let mins = secs / 60;
    secs %= 60;

    match style {
        DurationStyle::Clean => match mins {
            0 => format!("{secs:02}s"),
            _ => format!("{mins}m {secs:02}s"),
        },
        DurationStyle::CleanMillis => match mins {
            0 => format!("{secs:02}s {millis:03}ms"),
            _ => format!("{mins}m {secs:02}sec {millis:02}ms"),
        },
        DurationStyle::Compact => format!("{mins}:{secs:02}"),
        DurationStyle::CompactMillis => format!("{mins}:{secs:02}.{millis:02}"),
    }
}

fn truncate_at_last_space(s: &str, limit: usize) -> String {
    if s.chars().count() <= limit {
        return s.to_string();
    }

    let byte_limit = s
        .char_indices()
        .map(|(i, _)| i)
        .nth(limit)
        .unwrap_or(s.len());

    match s[..byte_limit].rfind(' ') {
        Some(last_space) => {
            let mut truncated = s[..last_space].to_string();
            truncated.push('…');
            truncated
        }
        None => {
            let char_boundary = s[..byte_limit]
                .char_indices()
                .map(|(i, _)| i)
                .last()
                .unwrap_or(0);

            let mut truncated = s[..char_boundary].to_string();
            truncated.push('…');
            truncated
        }
    }
}

pub fn strip_win_prefix(path: &str) -> String {
    let path_str = path.to_string();
    path_str
        .strip_prefix(r"\\?\")
        .unwrap_or(&path_str)
        .to_string()
}

pub fn overwrite_line(message: &str) {
    let mut stdout = std::io::stdout();
    stdout
        .execute(MoveToColumn(0))
        .unwrap()
        .execute(Clear(ClearType::CurrentLine))
        .unwrap()
        .execute(Print(message))
        .unwrap();
    stdout.flush().unwrap();
}

pub fn expand_tilde<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
    let path = path.as_ref();
    let path_str = path.to_string_lossy();

    if !path_str.starts_with('~') {
        return Ok(path.to_path_buf());
    }

    if path_str == "~" {
        return Err(anyhow!(
            "Setting the home directory would read every file in your system. Please provide a more specific path!"
        ));
    }

    if path_str.starts_with("~") || path_str.starts_with("~\\") {
        let home =
            dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory!"))?;
        return Ok(home.join(&path_str[2..]));
    }

    Err(anyhow!("Error reading directory with tilde (~)"))
}

pub fn get_random_playlist_idea() -> &'static str {
    use rand::seq::IndexedRandom;

    match PLAYLIST_IDEAS.choose(&mut rand::rng()) {
        Some(s) => s,
        None => "",
    }
}

const PLAYLIST_IDEAS: [&str; 46] = [
    "A Lantern in the Dark",
    "A Map Without Places",
    "After the Rain Ends",
    "Background Music for Poor Decisions",
    "Beats Me, Literally",
    "Certified Hood Classics (But It’s Just Me Singing)",
    "Chordially Yours",
    "Clouds Made of Static",
    "Coffee Shop Apocalypse",
    "Ctrl Alt Repeat",
    "Dancing on the Edge of Stillness",
    "Drifting Into Tomorrow",
    "Echoes Between Stars",
    "Existential Karaoke",
    "Fragments of a Dream",
    "Frequencies Between Worlds",
    "Ghosts of Tomorrow’s Sunlight",
    "Horizons That Never End",
    "I Liked It Before It Was Cool",
    "In Treble Since Birth",
    "Key Changes and Life Changes",
    "Liminal Grooves",
    "Low Effort High Vibes",
    "Major Minor Issues",
    "Melancholy But Make It Funky",
    "Microwave Symphony",
    "Midnight Conversations",
    "Music to Stare Dramatically Out the Window To",
    "Neon Memories in Sepia",
    "Note to Self",
    "Notes From Another Dimension",
    "Off-Brand Emotions™",
    "Rhythm & Clues",
    "Sharp Notes Only",
    "Silence Speaks Louder",
    "Songs Stuck Between Pages",
    "Songs That Owe Me Rent",
    "Soundtrack for Imaginary Films",
    "Tempo Tantrums",
    "Temporary Eternity",
    "The Shape of Sound to Come",
    "The Weight of Quiet",
    "Untranslatable Feelings",
    "Vinyl Countdown",
    "Waiting for the Beat to Drop (Forever)",
    "When the World Pauses",
];

// .\src\library\library.rs
use super::LEGAL_EXTENSION;
use crate::{
    calculate_signature,
    database::Database,
    domain::{Album, LongSong, SimpleSong, SongInfo},
    expand_tilde,
};
use anyhow::{Result, anyhow};
use indexmap::IndexMap;
use rayon::prelude::*;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::Arc,
};
use walkdir::WalkDir;

pub struct Library {
    db: Database,
    pub roots: HashSet<PathBuf>,
    pub songs: IndexMap<u64, Arc<SimpleSong>>,
    pub albums: Vec<Album>,
}

impl Library {
    fn new() -> Self {
        let db = Database::open().expect("Failed to connect to database!");
        Library {
            db,
            roots: HashSet::new(),
            songs: IndexMap::new(),
            albums: Vec::new(),
        }
    }

    pub fn init() -> Self {
        let mut lib = Self::new();

        {
            if let Ok(db_roots) = lib.db.get_roots() {
                for root in db_roots {
                    if let Ok(canon) = PathBuf::from(root).canonicalize() {
                        lib.roots.insert(canon);
                    }
                }
            }
        }

        lib
    }

    pub fn add_root(&mut self, root: impl AsRef<Path>) -> Result<()> {
        let expanded_path = expand_tilde(root.as_ref())?;
        let canon = PathBuf::from(expanded_path)
            .canonicalize()
            .map_err(|_| anyhow!("Path does not exist! {}", root.as_ref().display()))?;

        if self.roots.insert(canon.clone()) {
            self.db.set_root(&canon)?;
        }

        Ok(())
    }

    pub fn delete_root(&mut self, root: &str) -> Result<()> {
        let bad_root = PathBuf::from(root);
        match self.roots.remove(&bad_root) {
            true => self.db.delete_root(&bad_root),
            false => Err(anyhow!("Error deleting root")),
        }
    }

    /// Build the library based on the current state of the database.
    pub fn build_library(&mut self) -> Result<()> {
        if !self.roots.is_empty() {
            self.update_db_by_root()?;
            self.collect_songs()?;
            self.build_albums()?;
        }

        Ok(())
    }

    /// Walk through directories and update database based on changes made.
    pub fn update_db_by_root(&mut self) -> Result<(usize, usize)> {
        let mut existing_hashes = self.db.get_hashes()?;
        let mut new_files = Vec::new();

        for root in &self.roots {
            let files: Vec<PathBuf> = Self::collect_valid_files(root).collect();
            let new = Self::filter_files(files, &mut existing_hashes);
            new_files.extend(new);
        }

        let removed_ids = existing_hashes.into_iter().collect::<Vec<u64>>();
        let new_file_count = new_files.len();

        // WARNING: Flip these two if statements in the event that INSERT OR REPLACE fails us

        if !new_files.is_empty() {
            Self::insert_new_songs(&mut self.db, new_files)?;
        }

        if !removed_ids.is_empty() {
            self.db.delete_songs(&removed_ids)?;
        }

        Ok((new_file_count, removed_ids.len()))
    }

    /// Collect valid files from a root directory
    ///
    /// Function collects valid files with vetted extensions
    /// Currently, proper extensions are MP3, FLAC, and M4A
    ///
    /// Folders with a `.nomedia` file will be ignored
    fn collect_valid_files(dir: impl AsRef<Path>) -> impl ParallelIterator<Item = PathBuf> {
        WalkDir::new(dir)
            .into_iter()
            .filter_entry(|e| {
                !e.path().join(".nomedia").exists()
                    && !e.path().to_string_lossy().contains("$RECYCLE.BIN")
            })
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
            .collect::<Vec<_>>()
            .into_par_iter()
            .filter(move |entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| LEGAL_EXTENSION.contains(ext.to_lowercase().as_str()))
                    .unwrap_or(false)
            })
            .filter_map(|e| e.path().canonicalize().ok())
    }

    /// Attempt to remove hash from existing_hashes.
    /// If exists it will be removed, and no further processing
    /// is necessary
    ///
    /// If it cannot be removed, this indicates a file that may
    /// need to be processed
    ///
    /// Leftover hashes may indicate a file that has been updated,
    /// deleted, or can be found underneath other roots
    fn filter_files(all_paths: Vec<PathBuf>, existing_hashes: &mut HashSet<u64>) -> Vec<PathBuf> {
        all_paths
            .into_iter()
            .filter_map(|p| {
                let hash = calculate_signature(&p).unwrap();
                match existing_hashes.remove(&hash) {
                    true => None,
                    false => Some(p),
                }
            })
            .collect()
    }

    fn process_songs(paths: Vec<PathBuf>) -> Vec<LongSong> {
        paths
            .into_par_iter()
            .filter_map(|path| LongSong::build_song_symphonia(&path).ok())
            .collect::<Vec<LongSong>>()
    }

    fn insert_new_songs(db: &mut Database, new_files: Vec<PathBuf>) -> Result<()> {
        let songs = Self::process_songs(new_files);

        let mut artist_cache = HashSet::new();
        let mut aa_binding = HashSet::new();

        for song in &songs {
            // Artists and album_artists both included in the artist cache
            artist_cache.insert(song.get_artist());
            artist_cache.insert(song.album_artist.as_str());

            aa_binding.insert((song.album_artist.as_str(), song.get_album()));
        }

        // ORDER IS IMPORTANT HERE
        db.insert_artists(&artist_cache)?;
        db.insert_albums(&aa_binding)?;
        db.insert_songs(&songs)?;

        Ok(())
    }

    fn collect_songs(&mut self) -> Result<()> {
        self.songs = self.db.get_all_songs()?;
        Ok(())
    }

    pub fn get_songs_map(&self) -> &IndexMap<u64, Arc<SimpleSong>> {
        &self.songs
    }

    pub fn get_song_by_id(&self, id: u64) -> Option<&Arc<SimpleSong>> {
        self.songs.get(&id)
    }

    fn build_albums(&mut self) -> Result<()> {
        let aa_cache = self.db.get_album_map()?;
        self.albums = Vec::with_capacity(aa_cache.len());

        let mut album_lookup = HashMap::with_capacity(aa_cache.len());

        // Create album instances from album_artist/album_title combination
        for (album_name, artist_name) in &aa_cache {
            let album = Album::from_aa(album_name, artist_name);
            let idx = self.albums.len();
            self.albums.push(album);

            album_lookup.insert((Arc::clone(artist_name), Arc::clone(album_name)), idx);
        }

        // Assign each song to it's proper album
        for song in self.songs.values() {
            let key = (Arc::clone(&song.album_artist), Arc::clone(&song.album));

            let album_idx = match album_lookup.get(&key) {
                Some(&idx) => idx,
                None => {
                    let new_album = Album {
                        title: Arc::clone(&song.album),
                        artist: Arc::clone(&song.album_artist),
                        year: song.year,
                        tracklist: Vec::new(),
                    };
                    let idx = self.albums.len();
                    self.albums.push(new_album);
                    album_lookup.insert(key, idx);
                    idx
                }
            };

            let album = &mut self.albums[album_idx];
            if album.year.is_none() {
                album.year = song.year
            }

            album.tracklist.push(Arc::clone(song));
        }

        let mut bad_idx = vec![];
        for (idx, album) in self.albums.iter_mut().enumerate() {
            if album.tracklist.is_empty() {
                bad_idx.push(idx);
            }
            // Sort all tracks by disc number, then track number
            album
                .tracklist
                .sort_by_key(|s| (s.disc_no.unwrap_or(0), s.track_no.unwrap_or(0)));
        }

        // Because we may be removing multiple indexes, it's important to remove
        // each index from the back to the front. Earlier indexes will not be
        // affected by the removal of later indexes, but later indexes will be
        // affected by the removal of earlier indexes
        for idx in bad_idx.into_iter().rev() {
            self.albums.remove(idx);
        }

        Ok(())
    }
}

impl Library {
    pub fn set_history_db(&mut self, history: &[u64]) -> Result<()> {
        self.db.save_history_to_db(history)
    }

    pub fn load_history(
        &mut self,
        songs: &IndexMap<u64, Arc<SimpleSong>>,
    ) -> Result<VecDeque<Arc<SimpleSong>>> {
        self.db.import_history(songs)
    }
}

// UI State
impl Library {
    pub fn get_all_songs(&self) -> Vec<Arc<SimpleSong>> {
        self.songs.values().cloned().collect()
    }

    pub fn get_all_albums(&self) -> &[Album] {
        &self.albums
    }
}

// .\src\library\mod.rs
mod library;

pub use library::Library;

static LEGAL_EXTENSION: std::sync::LazyLock<std::collections::HashSet<&'static str>> =
    std::sync::LazyLock::new(|| {
        std::collections::HashSet::from(["mp3", "m4a", "flac", "ogg", "wav"])
    });

// .\src\library\progress.rs

// .\src\main.rs
fn main() -> anyhow::Result<()> {
    unsafe { std::env::set_var("RUST_BACKTRACE", "1") };
    concertus::app_core::Concertus::new().run()?;
    Ok(())
}

// .\src\player\controller.rs
use super::{PlaybackState, Player, PlayerCommand, PlayerState};
use crate::domain::{QueueSong, SimpleSong};
use anyhow::Result;
use std::{
    sync::{
        Arc, Mutex,
        mpsc::{self, Sender},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

pub struct PlayerController {
    sender: Sender<PlayerCommand>,
    shared_state: Arc<Mutex<PlayerState>>,
    _thread_handle: JoinHandle<()>,
}

impl PlayerController {
    pub fn new() -> Self {
        let (sender, reciever) = mpsc::channel();
        let shared_state = Arc::new(Mutex::new(PlayerState::default()));
        let shared_state_clone = Arc::clone(&shared_state);

        let thread_handle = thread::spawn(move || {
            let mut player = Player::new(shared_state_clone);

            loop {
                if let Ok(message) = reciever.try_recv() {
                    match message {
                        PlayerCommand::Play(song) => {
                            if let Err(e) = player.play_song(&song) {
                                let mut state = player.shared_state.lock().unwrap();

                                state.player_error = Some(e)
                            }
                        }
                        PlayerCommand::TogglePlayback => player.toggle_playback(),
                        PlayerCommand::SeekForward(secs) => {
                            player
                                .seek_forward(secs)
                                .unwrap_or_else(|e| eprintln!("Error: {e}"));
                        }
                        PlayerCommand::SeekBack(secs) => player.seek_back(secs),
                        PlayerCommand::Stop => player.stop(),
                    };
                }

                match player.sink_is_empty() {
                    true => player.stop(),
                    false => player.update_elapsed(),
                }
                // Lessen cpu intensity, but avoid stutters between songs
                thread::sleep(Duration::from_millis(10))
            }
        });

        PlayerController {
            sender,
            shared_state,
            _thread_handle: thread_handle,
        }
    }

    pub fn play_song(&self, song: Arc<QueueSong>) -> Result<()> {
        self.sender.send(PlayerCommand::Play(song))?;
        Ok(())
    }

    pub fn toggle_playback(&self) -> Result<()> {
        self.sender.send(PlayerCommand::TogglePlayback)?;
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        self.sender.send(PlayerCommand::Stop)?;
        Ok(())
    }

    pub fn seek_forward(&self, s: usize) -> Result<()> {
        self.sender.send(PlayerCommand::SeekForward(s))?;
        Ok(())
    }

    pub fn seek_back(&self, s: usize) -> Result<()> {
        self.sender.send(PlayerCommand::SeekBack(s))?;
        Ok(())
    }

    pub fn get_now_playing(&self) -> Option<Arc<SimpleSong>> {
        let state = self.shared_state.lock().unwrap();
        state.now_playing.clone()
    }

    /// Get the elapsed time of a song
    pub fn get_elapsed(&self) -> Duration {
        let state = self.shared_state.lock().unwrap();
        state.elapsed
    }

    pub fn is_paused(&self) -> bool {
        let state = self.shared_state.lock().unwrap();
        state.state == PlaybackState::Paused
    }

    pub fn sink_is_empty(&self) -> bool {
        let state = self.shared_state.lock().unwrap();
        state.now_playing.is_none() || state.state == PlaybackState::Stopped
    }

    pub fn get_shared_state(&self) -> Arc<Mutex<PlayerState>> {
        Arc::clone(&self.shared_state)
    }
}

// .\src\player\mod.rs
mod controller;
mod player;
mod state;

pub use controller::PlayerController;
pub use player::Player;
pub use state::{PlaybackState, PlayerState};

use crate::domain::QueueSong;
use std::sync::Arc;

pub enum PlayerCommand {
    Play(Arc<QueueSong>),
    TogglePlayback,
    SeekForward(usize),
    SeekBack(usize),
    Stop,
}

// .\src\player\player.rs
use super::{PlaybackState, PlayerState};
use crate::{
    domain::{QueueSong, SongInfo},
    get_readable_duration,
};
use anyhow::Result;
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, decoder::builder::SeekMode};
use std::{
    fs::File,
    io::BufReader,
    ops::Sub,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

pub struct Player {
    sink: Sink,
    pub shared_state: Arc<Mutex<PlayerState>>,
    _stream: OutputStream,
}

impl Player {
    pub(crate) fn new(shared_state: Arc<Mutex<PlayerState>>) -> Self {
        let _stream = OutputStreamBuilder::open_default_stream().expect("Cannot open stream");
        let sink = Sink::connect_new(_stream.mixer());

        Player {
            sink,
            shared_state,
            _stream,
        }
    }

    /// Play a song
    /// Returns an error if
    pub(crate) fn play_song(&mut self, song: &Arc<QueueSong>) -> Result<()> {
        let source = decode(song)?;

        self.sink.clear();
        self.sink.append(source);
        self.sink.play();

        let mut player_state = self
            .shared_state
            .lock()
            .expect("Failed to unwrap mutex in music player");
        player_state.state = PlaybackState::Playing;
        player_state.now_playing = Some(Arc::clone(&song.meta));
        player_state.elapsed = Duration::default();
        player_state.duration_display =
            get_readable_duration(song.meta.duration, crate::DurationStyle::Compact);
        player_state.elapsed_display = "0:00".to_string();

        Ok(())
    }

    /// Toggles the playback state of the audio player.
    ///
    /// This function manages the playback state transitions:
    /// - If no track is currently loaded (`now_playing` is None), it sets the state to `Stopped`.
    /// - If a track is loaded and currently paused, it resumes playback.
    /// - If a track is loaded and currently playing or in any other state, it pauses playback.
    ///
    /// # State Transitions
    /// - `None` -> `Stopped`
    /// - `Paused` -> `Playing` (resumes playback)
    /// - `Playing` or any other state -> `Paused`
    ///
    /// # Effects
    /// - When resuming, it calls `play()` on the sink.
    /// - When pausing, it calls `pause()` on the sink.
    ///
    /// # Examples
    /// ```
    /// let mut player = AudioPlayer::new();
    /// player.toggle_playback();       // Does nothing
    /// player.play_song(some_track);   // Starts playing
    /// player.toggle_playback();       // Pauses
    /// player.toggle_playback();       // Resumes playing
    /// ```
    pub(crate) fn toggle_playback(&mut self) {
        let (now_playing, playback_state) = {
            let state = self
                .shared_state
                .lock()
                .expect("Failed to unwrap mutex in music player");
            (state.now_playing.is_none(), state.state)
        };

        let mut state = self
            .shared_state
            .lock()
            .expect("Failed to unwrap mutex in music player");
        match (now_playing, playback_state) {
            (true, _) => state.state = PlaybackState::Stopped,

            //  RESUMING PLAYBACK
            (false, PlaybackState::Paused) => {
                self.sink.play();
                state.state = PlaybackState::Playing;
            }

            // PAUSING THE SINK
            (false, _) => {
                self.sink.pause();
                state.state = PlaybackState::Paused;
            }
        }
    }

    // /// Stop playback
    pub(crate) fn stop(&mut self) {
        self.sink.clear();

        let mut state = self
            .shared_state
            .lock()
            .expect("Failed to unwrap mutex in music player");
        state.now_playing = None;
        state.elapsed = Duration::default();
        state.state = PlaybackState::Stopped;
    }

    // BUG: Due to the development status of the symphonia crate, some decoders do not
    // implement seeking. FLAC files are dodgy, and often fail while testing in DEBUG
    // mode, however most problems seem to be solved in RELEASE mode. OGG files fail
    // with a 100% rate regardless of mode.
    // --
    // We'll try testing the symphonia 0.6 branch at some point to see how it fares.

    /// Fast forwards playback 5 seconds
    /// Will skip to next track if in last 5 seconds
    pub(crate) fn seek_forward(&mut self, secs: usize) -> Result<()> {
        let (now_playing, playback_state) = {
            let state = self
                .shared_state
                .lock()
                .expect("Failed to unwrap mutex in music player");
            (state.now_playing.clone(), state.state)
        };

        if playback_state != PlaybackState::Stopped {
            let elapsed = self.sink.get_pos();
            let duration = &now_playing.unwrap().duration;

            let mut state = self
                .shared_state
                .lock()
                .expect("Failed to unwrap mutex in music player");

            // This prevents skiping into the next song's playback
            if duration.sub(elapsed) > Duration::from_secs_f32(secs as f32 + 0.5) {
                let new_time = elapsed + Duration::from_secs(secs as u64);
                if let Err(_) = self.sink.try_seek(new_time) {
                    self.sink.clear();
                    state.state = PlaybackState::Stopped;
                } else {
                    state.elapsed = self.sink.get_pos()
                }
            } else {
                self.sink.clear();
                state.state = PlaybackState::Stopped;
            }
        }
        Ok(())
    }

    /// Rewinds playback 5 seconds
    pub(crate) fn seek_back(&mut self, secs: usize) {
        let playback_state = {
            let state = self
                .shared_state
                .lock()
                .expect("Failed to unwrap mutex in music player");
            state.state
        };

        if playback_state != PlaybackState::Stopped {
            let elapsed = self.sink.get_pos();

            match elapsed < Duration::from_secs(secs as u64) {
                true => {
                    let _ = self.sink.try_seek(Duration::from_secs(0));
                }
                false => {
                    let new_time = elapsed.sub(Duration::from_secs(secs as u64));
                    let _ = self.sink.try_seek(new_time);
                }
            }

            let mut state = self
                .shared_state
                .lock()
                .expect("Failed to unwrap mutex in music player");
            state.elapsed = self.sink.get_pos()
        }
    }

    pub(crate) fn update_elapsed(&self) {
        if let Ok(mut state) = self.shared_state.lock() {
            if state.state == PlaybackState::Playing {
                let new_elapsed = self.sink.get_pos();
                state.elapsed = new_elapsed;

                let secs = new_elapsed.as_secs();
                if secs != state.last_elapsed_secs {
                    state.last_elapsed_secs = secs;
                    state.elapsed_display =
                        get_readable_duration(new_elapsed, crate::DurationStyle::Compact);
                }
            }
        }
    }

    pub(crate) fn sink_is_empty(&self) -> bool {
        self.sink.empty()
    }
}

fn decode(song: &Arc<QueueSong>) -> Result<Decoder<BufReader<File>>> {
    let path = PathBuf::from(&song.path);
    let file = std::fs::File::open(&song.path)?;
    let duration = song.get_duration();

    let mut builder = Decoder::builder()
        .with_data(BufReader::new(file))
        .with_total_duration(duration)
        .with_seek_mode(SeekMode::Fastest)
        .with_seekable(true);

    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let hint = match ext {
            "adif" | "adts" => "aac",
            "caf" => "audio/x-caf",
            "m4a" | "m4b" | "m4p" | "m4r" | "mp4" => "audio/mp4",
            "bit" | "mpga" => "mp3",
            "mka" | "mkv" => "audio/matroska",
            "oga" | "ogm" | "ogv" | "ogx" | "spx" => "audio/ogg",
            "wave" => "wav",
            _ => ext,
        };
        builder = builder.with_hint(hint);
    }

    Ok(builder.build()?)
}

// .\src\player\state.rs
use crate::domain::SimpleSong;
use anyhow::Error;
use std::{sync::Arc, time::Duration};

pub struct PlayerState {
    pub now_playing: Option<Arc<SimpleSong>>,
    pub state: PlaybackState,
    pub elapsed: Duration,
    pub player_error: Option<Error>,

    pub elapsed_display: String,
    pub duration_display: String,
    pub last_elapsed_secs: u64,
}

impl Default for PlayerState {
    fn default() -> Self {
        PlayerState {
            now_playing: None,
            elapsed: Duration::default(),
            state: PlaybackState::Stopped,
            player_error: None,

            duration_display: String::new(),
            elapsed_display: String::new(),

            last_elapsed_secs: 0,
        }
    }
}

#[derive(PartialEq, Copy, Clone)]
pub enum PlaybackState {
    Playing,
    Paused,
    Stopped,
}

// .\src\tui\layout.rs
use crate::ui_state::{Mode, UiState};
use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct AppLayout {
    pub sidebar: Rect,
    pub search_bar: Rect,
    pub song_window: Rect,
    pub progress_bar: Rect,
    pub buffer_line: Rect,
}

impl AppLayout {
    pub fn new(area: Rect, state: &UiState) -> Self {
        let wf_height = match state.get_now_playing().is_some() {
            true => 7,
            false => 0,
        };

        let search_height = match state.get_mode() == Mode::Search {
            true => 5,
            false => 0,
        };

        let buffer_line_height = match !state.is_not_playing() || !state.bulk_select_empty() {
            true => 1,
            false => 0,
        };

        let [upper_block, progress_bar, buffer_line] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(12),
                Constraint::Length(wf_height),
                Constraint::Length(buffer_line_height),
            ])
            .areas(area);

        let [sidebar, _, upper_block] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Length(1),
                Constraint::Min(40),
            ])
            .areas(upper_block);

        let [search_bar, song_window] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(search_height), Constraint::Fill(100)])
            .areas(upper_block);

        AppLayout {
            sidebar,
            search_bar,
            song_window,
            progress_bar,
            buffer_line,
        }
    }
}

// .\src\tui\mod.rs
mod layout;
mod renderer;
mod widgets;

pub use layout::AppLayout;
pub use renderer::render;
pub use widgets::ErrorMsg;
pub use widgets::Progress;
pub use widgets::SearchBar;
pub use widgets::SideBarHandler as SideBar;
pub use widgets::SongTable;

use ratatui::widgets::Padding;
pub(crate) const SEARCH_PADDING: Padding = Padding {
    left: 2,
    right: 2,
    top: 1,
    bottom: 0,
};

// .\src\tui\renderer.rs
use super::widgets::Settings;
use super::{AppLayout, widgets::SongTable};
use super::{ErrorMsg, Progress, SearchBar, SideBar};
use crate::UiState;
use crate::tui::widgets::{BufferLine, PlaylistPopup};
use crate::ui_state::PopupType;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    widgets::{Widget, *},
};

pub fn render(f: &mut Frame, state: &mut UiState) {
    let layout = AppLayout::new(f.area(), &state);

    SearchBar.render(layout.search_bar, f.buffer_mut(), state);
    SideBar.render(layout.sidebar, f.buffer_mut(), state);
    SongTable.render(layout.song_window, f.buffer_mut(), state);
    Progress.render(layout.progress_bar, f.buffer_mut(), state);
    BufferLine.render(layout.buffer_line, f.buffer_mut(), state);

    if state.popup.is_open() {
        let popup_rect = match &state.popup.current {
            PopupType::Playlist(_) => centered_rect(35, 40, f.area()),
            PopupType::Settings(_) => centered_rect(35, 35, f.area()),
            PopupType::Error(_) => centered_rect(40, 30, f.area()),
            _ => centered_rect(30, 30, f.area()),
        };

        Clear.render(popup_rect, f.buffer_mut());
        match &state.popup.current {
            PopupType::Playlist(_) => PlaylistPopup.render(popup_rect, f.buffer_mut(), state),
            PopupType::Settings(_) => Settings.render(popup_rect, f.buffer_mut(), state),
            PopupType::Error(_) => ErrorMsg.render(popup_rect, f.buffer_mut(), state),
            _ => (),
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}

// .\src\tui\widgets\buffer_line.rs
use crate::{
    domain::SongInfo,
    truncate_at_last_space,
    tui::widgets::{PAUSE_ICON, SELECTED},
    ui_state::{DisplayTheme, GOLD_FADED, UiState},
};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Stylize,
    text::{Line, Span},
    widgets::{StatefulWidget, Widget},
};

pub struct BufferLine;

impl StatefulWidget for BufferLine {
    type State = UiState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let theme = state.get_theme(state.get_pane());

        let [left, center, right] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(40),
                Constraint::Percentage(30),
            ])
            .areas(area);

        let selection_count = state.get_bulk_sel().len();

        get_bulk_selection(selection_count).render(left, buf);
        playing_title(state, &theme, center.width as usize).render(center, buf);
        queue_display(state, &theme, right.width as usize).render(right, buf);
    }
}

fn playing_title(state: &UiState, theme: &DisplayTheme, width: usize) -> Option<Line<'static>> {
    let separator = match state.is_paused() {
        true => Span::from(format!(" {PAUSE_ICON} "))
            .fg(theme.text_focused)
            .rapid_blink(),
        false => Span::from(" ✧ ").fg(theme.text_faded),
    };

    if let Some(s) = state.get_now_playing() {
        let available_width = width.saturating_sub(3); // 3 is the length of " ✧ "

        let title = s.get_title();
        let artist = s.get_artist();

        let (final_title, final_artist) =
            if title.chars().count() + artist.chars().count() <= available_width {
                (title.to_string(), artist.to_string())
            } else if title.chars().count() <= available_width * 2 / 3 {
                // Title fits in 2/3, truncate artist
                let artist_space = available_width.saturating_sub(title.chars().count());
                (
                    title.to_string(),
                    truncate_at_last_space(artist, artist_space),
                )
            } else {
                let title_space = (available_width * 3) / 5;
                let artist_space = available_width.saturating_sub(title_space);
                (
                    truncate_at_last_space(title, available_width),
                    truncate_at_last_space(artist, artist_space),
                )
            };

        Some(
            Line::from_iter([
                Span::from(final_title).fg(theme.text_secondary),
                Span::from(separator).fg(theme.text_focused),
                Span::from(final_artist).fg(theme.text_faded),
            ])
            .centered(),
        )
    } else {
        None
    }
}

fn get_bulk_selection(size: usize) -> Option<Line<'static>> {
    let output = match size {
        0 => return None,
        x => format!("{x:>3} {} ", SELECTED)
            .fg(GOLD_FADED)
            .into_left_aligned_line(),
    };

    Some(output)
}

fn queue_display(state: &UiState, theme: &DisplayTheme, width: usize) -> Option<Line<'static>> {
    let up_next = state.peek_queue()?;

    let alert = state
        .get_now_playing()
        .map(|np| {
            let duration = np.duration.as_secs_f32();
            let elapsed = state.get_playback_elapsed().as_secs_f32();

            (duration - elapsed) < 3.0
        })
        .unwrap_or(false);

    let up_next_str = up_next.get_title();
    let truncated = truncate_at_last_space(up_next_str, width - 12);
    let total = state.playback.queue.len();

    let up_next_line = match alert {
        true => Span::from(truncated).fg(GOLD_FADED).rapid_blink(),
        false => Span::from(truncated).fg(GOLD_FADED),
    };

    Some(
        Line::from_iter([
            Span::from("Up next ✧ ").fg(theme.text_faded),
            up_next_line,
            format!(" [{total}] ").fg(theme.text_faded),
        ])
        .right_aligned(),
    )
}

// .\src\tui\widgets\error.rs
use crate::ui_state::UiState;
use ratatui::{
    layout::{Alignment, Constraint, Layout},
    style::Stylize,
    widgets::{Block, BorderType, Padding, Paragraph, StatefulWidget, Widget, Wrap},
};

static SIDE_PADDING: u16 = 5;
static VERTICAL_PADDING: u16 = 1;

static PADDING: Padding = Padding {
    left: SIDE_PADDING,
    right: SIDE_PADDING,
    top: VERTICAL_PADDING,
    bottom: VERTICAL_PADDING,
};

pub struct ErrorMsg;
impl StatefulWidget for ErrorMsg {
    type State = UiState;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let block = Block::bordered()
            .border_type(BorderType::Double)
            .title_bottom(" Press <Esc> to clear ")
            .title_alignment(Alignment::Center)
            .padding(PADDING)
            .bg(ratatui::style::Color::LightRed);

        let inner = block.inner(area);
        block.render(area, buf);
        let chunks = Layout::vertical([
            Constraint::Percentage(33),
            Constraint::Length(3),
            Constraint::Fill(1),
        ])
        .split(inner);

        let err_str = state.get_error().unwrap_or("No error to display");

        Paragraph::new(err_str)
            .wrap(Wrap { trim: true })
            .centered()
            .render(chunks[1], buf);
    }
}

// .\src\tui\widgets\mod.rs
mod buffer_line;
mod error;
mod playlist_popup;
mod progress;
mod root_mgmt;
mod search;
mod sidebar;
mod song_window;
mod tracklist;

pub use buffer_line::BufferLine;
pub use error::ErrorMsg;
pub use playlist_popup::PlaylistPopup;
pub use progress::Progress;
pub use root_mgmt::Settings;
pub use search::SearchBar;
pub use sidebar::SideBarHandler;
pub use song_window::SongTable;

const DUR_WIDTH: u16 = 5;
const PAUSE_ICON: &str = "󰏤";
const SELECTOR: &str = "⮞  ";
const MUSIC_NOTE: &str = "♫";
const QUEUED: &str = "";
const SELECTED: &str = "󱕣";
const DECORATOR: &str = " ♠ ";
const WAVEFORM_WIDGET_HEIGHT: f64 = 50.0;

static POPUP_PADDING: ratatui::widgets::Padding = ratatui::widgets::Padding {
    left: 5,
    right: 5,
    top: 2,
    bottom: 1,
};

// .\src\tui\widgets\playlist_popup.rs
use crate::{
    tui::widgets::POPUP_PADDING,
    ui_state::{GOLD, PlaylistAction, PopupType, UiState},
};
use ratatui::{
    layout::{Alignment, Constraint, Layout},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, List, Padding, Paragraph, StatefulWidget, Widget, Wrap},
};

pub struct PlaylistPopup;
impl StatefulWidget for PlaylistPopup {
    type State = UiState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        if let PopupType::Playlist(action) = &state.popup.current {
            match action {
                PlaylistAction::Create => render_create_popup(area, buf, state),
                PlaylistAction::AddSong => render_add_song_popup(area, buf, state),
                PlaylistAction::Delete => render_delete_popup(area, buf, state),
                PlaylistAction::Rename => render_rename_popup(area, buf, state),
            }
        }
    }
}

fn render_create_popup(
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
    state: &mut UiState,
) {
    let block = Block::bordered()
        .title(" Create New Playlist ")
        .title_bottom(" [Enter] confirm / [Esc] cancel ")
        .title_alignment(ratatui::layout::Alignment::Center)
        .border_type(BorderType::Double)
        .border_style(Style::new().fg(Color::Rgb(255, 70, 70)))
        .bg(Color::Rgb(25, 25, 25))
        .padding(POPUP_PADDING);

    let inner = block.inner(area);
    block.render(area, buf);

    let chunks = Layout::vertical([
        Constraint::Max(2),
        Constraint::Max(2),
        Constraint::Length(3),
    ])
    .split(inner);

    Paragraph::new("Enter a name for your new playlist:")
        .centered()
        .render(chunks[1], buf);

    state.popup.input.set_block(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .fg(Color::Rgb(220, 220, 100))
            .padding(Padding::horizontal(1)),
    );
    state.popup.input.set_style(Style::new().fg(Color::White));
    state.popup.input.render(chunks[2], buf);
}

fn render_add_song_popup(
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
    state: &mut UiState,
) {
    let list_items = state
        .playlists
        .iter()
        .map(|p| {
            let playlist_name = p.name.to_string();
            Line::from(playlist_name)
        })
        .collect::<Vec<Line>>();

    let block = Block::bordered()
        .title(" Select Playlist ")
        .title_bottom(" [Enter] confirm / [Esc] cancel ")
        .title_alignment(ratatui::layout::Alignment::Center)
        .border_type(BorderType::Double)
        .border_style(Style::new().fg(Color::Rgb(255, 70, 70)))
        .bg(Color::Rgb(25, 25, 25))
        .padding(POPUP_PADDING);

    let list = List::new(list_items)
        .block(block)
        .highlight_style(Style::new().fg(Color::Black).bg(GOLD));
    ratatui::prelude::StatefulWidget::render(list, area, buf, &mut state.popup.selection);
}

fn render_delete_popup(
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
    state: &mut UiState,
) {
    let block = Block::bordered()
        .title(format!(" Delete Playlist?? "))
        .title_bottom(" [Enter] confirm / [Esc] cancel ")
        .title_alignment(ratatui::layout::Alignment::Center)
        .border_type(BorderType::Double)
        .border_style(Style::new().fg(Color::Rgb(255, 70, 70)))
        .bg(Color::Rgb(25, 25, 25))
        .padding(POPUP_PADDING);

    if let Some(p) = state.get_selected_playlist() {
        let warning = Paragraph::new(format!("Are you sure you want to delete\n[{}]?", p.name))
            .block(block)
            .wrap(Wrap { trim: true })
            .centered();
        warning.render(area, buf);
    };
}

fn render_rename_popup(
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
    state: &mut UiState,
) {
    let block = Block::bordered()
        .title(" Rename Playlist ")
        .title_bottom(" [Enter] confirm / [Esc] cancel ")
        .title_alignment(Alignment::Center)
        .border_type(BorderType::Double)
        .border_style(Style::new().fg(Color::Rgb(255, 70, 70)))
        .bg(Color::Rgb(25, 25, 25))
        .padding(POPUP_PADDING);

    let inner = block.inner(area);
    block.render(area, buf);

    let chunks = Layout::vertical([
        Constraint::Percentage(10),
        Constraint::Max(3),
        Constraint::Length(3),
        Constraint::Fill(1),
    ])
    .split(inner);

    if let Some(playlist) = state.get_selected_playlist() {
        Paragraph::new(format!("Enter a new name for\n `[{}]`: ", playlist.name))
            .centered()
            .render(chunks[1], buf);

        state.popup.input.set_block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .fg(Color::Rgb(220, 220, 100))
                .padding(Padding::horizontal(1)),
        );

        state.popup.input.set_style(Style::new().fg(Color::White));
        state.popup.input.render(chunks[2], buf);
    }
}

// .\src\tui\widgets\progress\mod.rs
mod waveform;
use ratatui::widgets::StatefulWidget;

use crate::{tui::widgets::progress::waveform::Waveform, ui_state::UiState};

pub struct Progress;
impl StatefulWidget for Progress {
    type State = UiState;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        if state.get_now_playing().is_some() {
            Waveform.render(area, buf, state);
        }
    }
}

// .\src\tui\widgets\progress\progress_bar.rs
use super::PAUSE_ICON;
use crate::{domain::SongInfo, get_readable_duration, ui_state::UiState, DurationStyle};
use ratatui::{
    layout::Alignment,
    style::{Color, Stylize},
    symbols,
    text::{Line, Span},
    widgets::{Block, LineGauge, Padding, StatefulWidget, Widget},
};

pub struct ProgressBar;

impl StatefulWidget for ProgressBar {
    type State = UiState;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let np = state
            .get_now_playing()
            .expect("Expected a song to be playing. [Widget: Progress Bar]");
        let elapsed = state.get_playback_elapsed();
        let duration = np.get_duration().as_secs_f32();
        let progress_raw = elapsed.as_secs_f32() / duration;
        // let theme = &state.get_theme(&Pane::TrackList);

        // The program will crash if this hit's 1.0
        let ratio = match progress_raw {
            i if i < 1.0 => i,
            _ => 0.0,
        };

        let is_paused = state.is_paused().then(|| PAUSE_ICON).unwrap_or("");

        // BUG: This label creation MAY cause the search
        // cursor to flicker indcredibly quickly
        // specifically in the Windows terminal/cmd
        let label = match state.is_not_playing() {
            true => "0:00.00 / 0:00".into(),
            false => {
                format!(
                    "{:1} {} / {}", // :1 Prevents shift in widget when pause icon appears
                    is_paused,
                    get_readable_duration(elapsed, DurationStyle::Compact),
                    get_readable_duration(np.get_duration(), DurationStyle::Compact),
                )
            }
        };

        let playing_title = Line::from_iter([
            Span::from(np.get_title()).fg(Color::Red),
            Span::from(" ✧ ").fg(Color::DarkGray),
            Span::from(np.get_artist()).fg(Color::Gray),
        ]);

        let guage = LineGauge::default()
            .block(
                Block::new()
                    .title_top(playing_title.alignment(Alignment::Center))
                    .padding(Padding {
                        left: 10,
                        right: 10,
                        top: 2,
                        bottom: 0,
                    }),
            )
            .filled_style(ratatui::style::Color::Magenta)
            .line_set(symbols::line::THICK)
            .label(label)
            .ratio(ratio as f64);

        guage.render(area, buf);
    }
}

// .\src\tui\widgets\progress\waveform.rs
use crate::{
    domain::SongInfo,
    tui::widgets::{DUR_WIDTH, WAVEFORM_WIDGET_HEIGHT},
    ui_state::UiState,
};
use canvas::Context;
use ratatui::{
    layout::Rect,
    style::{Color, Stylize},
    text::Text,
    widgets::{
        StatefulWidget,
        canvas::{Canvas, Rectangle},
        *,
    },
};

pub struct Waveform;
impl StatefulWidget for Waveform {
    type State = UiState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let np = state
            .get_now_playing()
            .expect("Expected a song to be playing. [Widget: Waveform]");

        let waveform = state.get_waveform_visual();
        let wf_len = waveform.len();

        let x_duration = area.width - 8;
        let y = buf.area().height
            - match area.height {
                0 => 1,
                _ => area.height / 2 + 2,
            };

        let player_state = state.playback.player_state.lock().unwrap();
        let elapsed_str = player_state.elapsed_display.as_str();
        let duration_str = player_state.duration_display.as_str();

        Text::from(elapsed_str)
            .fg(Color::DarkGray)
            .right_aligned()
            .render(Rect::new(2, y, DUR_WIDTH, 1), buf);

        Text::from(duration_str)
            .fg(Color::DarkGray)
            .right_aligned()
            .render(Rect::new(x_duration, y, DUR_WIDTH, 1), buf);

        // PREVENT DEADLOCKS
        drop(player_state);

        Canvas::default()
            .x_bounds([0.0, wf_len as f64])
            .y_bounds([WAVEFORM_WIDGET_HEIGHT * -1.0, WAVEFORM_WIDGET_HEIGHT])
            .paint(|ctx| {
                let duration_f32 = &np.get_duration_f32();
                let elapsed = &state.get_playback_elapsed();

                let progress = elapsed.as_secs_f32() / duration_f32;
                let line_mode = area.width < 170;

                for (idx, amp) in waveform.iter().enumerate() {
                    let hgt = (*amp as f64 * WAVEFORM_WIDGET_HEIGHT).round();
                    let color = match (idx as f32 / wf_len as f32) < progress {
                        true => Color::Rgb(170, 0, 170),
                        false => Color::default(),
                    };

                    match line_mode {
                        true => draw_waveform_line(ctx, idx as f64, hgt, color),
                        false => draw_waveform_rect(ctx, idx as f64, hgt, color),
                    }
                }
            })
            .block(Block::new().padding(Padding {
                left: 10,
                right: 10,
                top: 1,
                bottom: 1,
            }))
            .render(area, buf)
    }
}

/// Lines create a more detailed and cleaner look
/// especially when seen in smaller windows
fn draw_waveform_line(ctx: &mut Context, idx: f64, hgt: f64, color: Color) {
    ctx.draw(&canvas::Line {
        x1: idx,
        x2: idx,
        y1: hgt,
        y2: hgt * -1.0,
        color,
    })
}

/// Rectangles cleanly extend the waveform when in
/// full-screen view
fn draw_waveform_rect(ctx: &mut Context, idx: f64, hgt: f64, color: Color) {
    ctx.draw(&Rectangle {
        x: idx as f64,
        y: hgt * -1.0,
        width: f64::from(0.5), // This value makes the waveform cleaner on resize
        height: hgt * 2.0,
        color,
    });
}

// .\src\tui\widgets\root_mgmt.rs
use crate::{
    strip_win_prefix,
    tui::widgets::POPUP_PADDING,
    ui_state::{GOOD_RED, SettingsMode, UiState},
};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{
        Block, BorderType, HighlightSpacing, List, Padding, Paragraph, StatefulWidget, Widget, Wrap,
    },
};

pub struct Settings;
impl StatefulWidget for Settings {
    type State = UiState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let settings_mode = state.get_settings_mode();

        let title = match settings_mode {
            Some(SettingsMode::ViewRoots) => " Settings - Music Library Roots ",
            Some(SettingsMode::AddRoot) => " Add New Root Directory ",
            Some(SettingsMode::RemoveRoot) => " Remove Root Directory ",
            None => return,
        };

        let block = Block::bordered()
            .title(title)
            .title_bottom(get_help_text(settings_mode))
            .title_alignment(ratatui::layout::Alignment::Center)
            .border_type(BorderType::Double)
            .border_style(Style::new().fg(Color::Rgb(255, 70, 70)))
            .bg(Color::Rgb(25, 25, 25))
            .padding(POPUP_PADDING);

        let inner = block.inner(area);
        block.render(area, buf);

        match settings_mode {
            Some(SettingsMode::ViewRoots) => render_roots_list(inner, buf, state),
            Some(SettingsMode::AddRoot) => render_add_root(inner, buf, state),
            Some(SettingsMode::RemoveRoot) => render_remove_root(inner, buf, state),
            None => (),
        }
    }
}

fn get_help_text(mode: Option<&SettingsMode>) -> &'static str {
    if let Some(m) = mode {
        match m {
            SettingsMode::ViewRoots => " [a]dd / [d]elete / [Esc] close ",
            SettingsMode::AddRoot => " [Enter] confirm / [Esc] cancel ",
            SettingsMode::RemoveRoot => " [Enter] confirm / [Esc] cancel ",
        }
    } else {
        unreachable!()
    }
}

fn render_roots_list(
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
    state: &mut UiState,
) {
    let roots = state.get_roots();

    if roots.is_empty() {
        Paragraph::new("No music library configured.\nPress 'a' to add a parent directory.")
            .wrap(Wrap { trim: true })
            .centered()
            .render(area, buf);
        return;
    }

    let items: Vec<Line> = roots
        .iter()
        .map(|r| {
            let root = strip_win_prefix(r);
            Line::from(root)
        })
        .collect();

    let theme = state.get_theme(state.get_pane());

    let list = List::new(items)
        .highlight_style(Style::default().fg(Color::Black).bg(theme.text_highlighted))
        .highlight_spacing(HighlightSpacing::Always);

    ratatui::prelude::StatefulWidget::render(list, area, buf, &mut state.popup.selection);
}

fn render_add_root(
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
    state: &mut UiState,
) {
    let chunks = Layout::vertical([
        Constraint::Max(3),
        Constraint::Length(3),
        Constraint::Fill(1),
    ])
    .split(area);

    Paragraph::new("Enter the path to a directory containing music files:")
        .wrap(Wrap { trim: false })
        .render(chunks[0], buf);

    let theme = state.get_theme(state.get_pane());

    state.popup.input.set_block(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .fg(theme.text_highlighted)
            .padding(Padding {
                left: 1,
                right: 1,
                top: 0,
                bottom: 0,
            }),
    );
    state
        .popup
        .input
        .set_style(Style::new().fg(theme.text_focused));

    state.popup.input.render(chunks[1], buf);

    let example = Paragraph::new("Example: C:\\Music or /home/user/music")
        .fg(Color::DarkGray)
        .centered();
    example.render(chunks[2], buf);
}

fn render_remove_root(
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
    state: &UiState,
) {
    let roots = state.get_roots();

    if roots.is_empty() {
        Paragraph::new("No root selected")
            .centered()
            .render(area, buf);
        return;
    }
    let selected_root = &roots[state.popup.selection.selected().unwrap()];
    let selected_root = strip_win_prefix(&selected_root);

    let warning = Paragraph::new(format!(
        "Are you sure you want to remove this root?\n\n{}\n\nThis will remove all songs from this directory from your library.",
        selected_root
    ))
    .wrap(Wrap { trim: true })
    .centered()
    .fg(GOOD_RED);

    warning.render(area, buf);
}

// .\src\tui\widgets\search.rs
use crate::{
    tui::SEARCH_PADDING,
    ui_state::{Pane, UiState},
};
use ratatui::{
    style::Stylize,
    widgets::{Block, BorderType, StatefulWidget, Widget},
};

pub struct SearchBar;

impl StatefulWidget for SearchBar {
    type State = UiState;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let theme = &state.get_theme(&Pane::Search);
        let search = state.get_search_widget();
        search.set_block(
            Block::bordered()
                .border_type(BorderType::Thick)
                .border_style(theme.border)
                .padding(SEARCH_PADDING)
                .fg(theme.text_highlighted),
        );

        search.render(area, buf);
    }
}

// .\src\tui\widgets\sidebar\album_sidebar.rs
use crate::ui_state::{AlbumSort, Pane, UiState, GOLD_FADED};
use ratatui::{
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{
        Block, BorderType, HighlightSpacing, List, ListItem, ListState, Padding, StatefulWidget,
    },
};

// album_view.rs
pub struct SideBarAlbum;
impl StatefulWidget for SideBarAlbum {
    type State = UiState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let theme = &state.get_theme(&Pane::SideBar);

        let albums = &state.albums;
        let pane_sort = state.get_album_sort_string();
        let pane_sort = format!("{pane_sort:5} ");

        let selected_album_idx = state.display_state.album_pos.selected();
        let selected_artist = state.get_selected_album().map(|a| a.artist.as_str());

        let mut list_items = Vec::new();
        let mut current_artist = None;
        let mut current_display_idx = 0;
        let mut selected_display_idx = None;

        for (idx, album) in albums.iter().enumerate() {
            // Add header if artist changed (only for Artist sort)
            if state.get_album_sort() == AlbumSort::Artist {
                if current_artist.as_ref() != Some(&album.artist.as_str()) {
                    let artist_str = album.artist.as_str();
                    let is_selected_artist = selected_artist == Some(artist_str);

                    // Match header style to selected album
                    let header_style = match is_selected_artist {
                        true => Style::default()
                            .fg(theme.text_highlighted)
                            .italic()
                            .underlined(),
                        false => Style::default().fg(GOLD_FADED),
                    };

                    list_items.push(ListItem::new(Span::from(artist_str).style(header_style)));

                    current_artist = Some(artist_str);
                    current_display_idx += 1;
                }
            }

            // Build album item
            let year = album.year.map_or("----".to_string(), |y| format!("{y}"));

            let indent = match state.get_album_sort() == AlbumSort::Artist {
                true => "  ",
                false => "",
            };

            let is_selected = selected_album_idx == Some(idx);
            if is_selected {
                selected_display_idx = Some(current_display_idx);
            }

            // Don't apply selection styling here - let the List widget handle it
            list_items.push(ListItem::new(Line::from_iter([
                Span::from(format!("{}{: >4} ", indent, year)).fg(theme.text_secondary),
                Span::from("✧ ").fg(theme.text_faded),
                Span::from(album.title.as_str()).fg(theme.text_focused),
            ])));

            current_display_idx += 1;
        }

        // Temp state for rendering with display index
        let mut render_state = ListState::default();
        render_state.select(selected_display_idx);

        // Sync offset to ensure selection is visible
        if let Some(idx) = selected_display_idx {
            let current_offset = state.display_state.album_pos.offset();
            let visible_height = area.height.saturating_sub(4) as usize;

            if idx < current_offset {
                *render_state.offset_mut() = idx;
            } else if idx >= current_offset + visible_height {
                *render_state.offset_mut() = idx.saturating_sub(visible_height - 1);
            } else {
                *render_state.offset_mut() = current_offset;
            }
        }

        let keymaps = match state.get_pane() {
            Pane::SideBar => Line::from(" [q] Queue Album ")
                .centered()
                .fg(theme.text_faded),
            _ => Line::default(),
        };

        let block = Block::bordered()
            .borders(theme.border_display)
            .border_type(BorderType::Thick)
            .border_style(theme.border)
            .bg(theme.bg)
            .title_top(format!(" ⟪ {} Albums! ⟫ ", albums.len()))
            .title_top(
                Line::from_iter([" 󰒿 ", &pane_sort])
                    .right_aligned()
                    .fg(theme.text_secondary),
            )
            .title_bottom(Line::from(keymaps).centered().fg(theme.text_faded))
            .padding(Padding {
                left: 3,
                right: 4,
                top: 1,
                bottom: 1,
            });

        let list = List::new(list_items)
            .block(block)
            .highlight_style(
                Style::new()
                    .fg(Color::Black)
                    .bg(theme.text_highlighted)
                    .italic(),
            )
            .scroll_padding(5)
            .highlight_spacing(HighlightSpacing::Always);

        list.render(area, buf, &mut render_state);

        // Sync offset back
        *state.display_state.album_pos.offset_mut() = render_state.offset();
    }
}

// .\src\tui\widgets\sidebar\handler.rs
use super::{SideBarAlbum, SideBarPlaylist};
use crate::ui_state::{LibraryView, UiState};
use ratatui::widgets::StatefulWidget;

pub struct SideBarHandler;
impl StatefulWidget for SideBarHandler {
    type State = UiState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        match state.get_sidebar_view() {
            LibraryView::Albums => SideBarAlbum.render(area, buf, state),
            LibraryView::Playlists => SideBarPlaylist.render(area, buf, state),
        }
    }
}

// .\src\tui\widgets\sidebar\mod.rs
mod album_sidebar;
mod handler;
mod playlist_sidebar;

pub use album_sidebar::SideBarAlbum;
pub use handler::SideBarHandler;
pub use playlist_sidebar::SideBarPlaylist;

// .\src\tui\widgets\sidebar\playlist_sidebar.rs
use ratatui::{
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, List, ListItem, Padding, StatefulWidget},
};

use crate::ui_state::{Pane, UiState, GOLD_FADED};

pub struct SideBarPlaylist;
impl StatefulWidget for SideBarPlaylist {
    type State = UiState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let theme = &state.get_theme(&Pane::SideBar);
        let playlists = &state.playlists;

        let list_items = playlists.iter().map(|p| {
            ListItem::new(
                Line::from_iter([
                    Span::from(p.name.as_str()).fg(theme.text_secondary),
                    format!("{:>5} ", format!("[{}]", p.tracklist.len()))
                        .fg(GOLD_FADED)
                        .into(),
                ])
                .right_aligned(),
            )
        });

        let keymaps = match state.get_pane() {
            Pane::SideBar => Line::from(" [c]reate 󰲸 | [D]elete 󰐓 ")
                .centered()
                .fg(theme.text_faded),
            _ => Line::default(),
        };

        let block = Block::bordered()
            .border_type(BorderType::Thick)
            .border_style(theme.border)
            .bg(theme.bg)
            .title_top(
                Line::from(format!(" ⟪ {} Playlists! ⟫ ", playlists.len()))
                    .left_aligned()
                    .fg(theme.text_highlighted),
            )
            .title_bottom(Line::from(keymaps).centered().fg(theme.text_faded))
            .padding(Padding {
                left: 3,
                right: 4,
                top: 1,
                bottom: 1,
            });

        let list = List::new(list_items)
            .block(block)
            .highlight_style(
                Style::new()
                    .fg(Color::Black)
                    .bg(theme.text_highlighted)
                    .italic(),
            )
            .scroll_padding(4);

        list.render(area, buf, &mut state.display_state.playlist_pos);
    }
}

// .\src\tui\widgets\song_window.rs
use super::tracklist::{AlbumView, StandardTable};
use crate::{
    tui::widgets::tracklist::GenericView,
    ui_state::{LibraryView, Mode, UiState},
};
use ratatui::widgets::StatefulWidget;

pub struct SongTable;
impl StatefulWidget for SongTable {
    type State = UiState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        match state.get_mode() {
            &Mode::Library(LibraryView::Albums) => AlbumView.render(area, buf, state),
            &Mode::Library(LibraryView::Playlists) | &Mode::Queue => {
                GenericView.render(area, buf, state)
            }
            _ => StandardTable.render(area, buf, state),
        }
    }
}

// .\src\tui\widgets\tracklist\album_tracklist.rs
use crate::{
    truncate_at_last_space,
    tui::widgets::tracklist::{CellFactory, create_empty_block, create_standard_table},
    ui_state::{Pane, UiState},
};
use ratatui::{
    style::Stylize,
    text::{Line, Span},
    widgets::{Row, StatefulWidget, Widget},
};

pub struct AlbumView;
impl StatefulWidget for AlbumView {
    type State = UiState;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let theme = &state.get_theme(&Pane::TrackList);

        if state.albums.is_empty() {
            create_empty_block(theme, "0 Songs").render(area, buf);
            return;
        }

        let album = state.get_selected_album().unwrap_or(&state.albums[0]);
        let album_title = truncate_at_last_space(&album.title, (area.width / 3) as usize);

        let rows = album
            .tracklist
            .iter()
            .map(|song| {
                let track_no = CellFactory::get_track_discs(theme, song);
                let icon = CellFactory::status_cell(song, state);
                let title = CellFactory::title_cell(theme, song);
                let artist = CellFactory::artist_cell(theme, song);
                let format = CellFactory::filetype_cell(theme, song);
                let duration = CellFactory::duration_cell(theme, song);

                Row::new([track_no, icon, title.into(), artist, format, duration])
            })
            .collect::<Vec<Row>>();

        let year_str = album
            .year
            .filter(|y| *y != 0)
            .map_or(String::new(), |y| format!("[{y}]"));

        let title = Line::from_iter([
            Span::from(format!(" {} ", album_title))
                .fg(theme.text_secondary)
                .italic(),
            Span::from(year_str).fg(theme.text_faded),
            Span::from(" ✧ ").fg(theme.text_faded),
            Span::from(album.artist.to_string()).fg(theme.text_highlighted),
            Span::from(format!(" [{} Songs] ", album.tracklist.len())).fg(theme.text_faded),
        ]);

        let table = create_standard_table(rows, title, state);
        StatefulWidget::render(table, area, buf, &mut state.display_state.table_pos);
    }
}

// .\src\tui\widgets\tracklist\generic_tracklist.rs
use crate::{
    tui::widgets::tracklist::{create_standard_table, get_title, CellFactory},
    ui_state::{Pane, UiState},
};
use ratatui::widgets::{Row, StatefulWidget};

pub struct GenericView;
impl StatefulWidget for GenericView {
    type State = UiState;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let theme = &state.get_theme(&Pane::TrackList);
        let songs = state.legal_songs.as_slice();

        let rows = songs
            .iter()
            .enumerate()
            .map(|(idx, song)| {
                let index = CellFactory::index_cell(&theme, idx);
                let icon = CellFactory::status_cell(song, state);
                let title = CellFactory::title_cell(&theme, song);
                let artist = CellFactory::artist_cell(&theme, song);
                let filetype = CellFactory::filetype_cell(&theme, song);
                let duration = CellFactory::duration_cell(&theme, song);

                Row::new([index, icon, title, artist, filetype, duration])
            })
            .collect::<Vec<Row>>();

        let title = get_title(state, area);

        let table = create_standard_table(rows, title.into(), state);
        StatefulWidget::render(table, area, buf, &mut state.display_state.table_pos);
    }
}

// .\src\tui\widgets\tracklist\mod.rs
mod album_tracklist;
mod generic_tracklist;
mod search_results;

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, LazyLock},
};

pub use album_tracklist::AlbumView;
pub use generic_tracklist::GenericView;
pub use search_results::StandardTable;

use crate::{
    DurationStyle,
    domain::{SimpleSong, SongInfo},
    get_readable_duration,
    tui::widgets::{DECORATOR, MUSIC_NOTE, QUEUED, SELECTED, SELECTOR},
    ui_state::{DisplayTheme, LibraryView, Mode, Pane, TableSort, UiState},
};
use ratatui::{
    layout::{Alignment, Constraint, Flex, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Cell, HighlightSpacing, Padding, Row, Table},
};

const COLUMN_SPACING: u16 = 2;

const PADDING: Padding = Padding {
    left: 2,
    right: 3,
    top: 1,
    bottom: 1,
};

pub(super) fn get_widths(mode: &Mode) -> Vec<Constraint> {
    match mode {
        Mode::Power | Mode::Search => {
            vec![
                Constraint::Length(1),
                Constraint::Ratio(3, 9),
                Constraint::Ratio(2, 9),
                Constraint::Ratio(2, 9),
                Constraint::Length(8),
            ]
        }
        Mode::Library(_) | Mode::Queue => {
            vec![
                Constraint::Length(6),
                Constraint::Length(1),
                Constraint::Min(25),
                Constraint::Max(20),
                Constraint::Max(4),
                Constraint::Length(7),
            ]
        }
        _ => Vec::new(),
    }
}

pub(super) fn get_header<'a>(state: &UiState, active: &TableSort) -> Row<'a> {
    let row = match state.get_mode() {
        Mode::Power | Mode::Search => [
            String::new(),
            TableSort::Title.to_string(),
            TableSort::Artist.to_string(),
            TableSort::Album.to_string(),
            TableSort::Duration.to_string(),
        ]
        .iter()
        .map(
            |s| match (*s == active.to_string(), s.eq(&String::from("Duration"))) {
                (true, true) => Text::from(s.to_string())
                    .fg(Color::Red)
                    .underlined()
                    .italic()
                    .right_aligned(),
                (false, true) => Text::from(s.to_string()).right_aligned(),
                (true, false) => Text::from(Span::from(
                    s.to_string().fg(Color::Red).underlined().italic(),
                )),
                _ => s.to_string().into(),
            },
        )
        .collect(),
        Mode::Library(_) | Mode::Queue => {
            vec![
                Text::default(),
                Text::default(),
                Text::from("𝕋𝕚𝕥𝕝𝕖").bold(),
                Text::from("𝔸𝕣𝕥𝕚𝕤𝕥"),
                Text::from("").centered(),
                Text::from("").centered(),
            ]
        }
        _ => Vec::new(),
    };

    Row::new(row).bottom_margin(1).bold()
}

pub fn get_keymaps(mode: &Mode) -> &'static str {
    matches!(mode, Mode::Library(LibraryView::Playlists) | Mode::Queue)
        .then_some(" [q]ueue ✧ [a]dd to playlist ✧ [x] remove ")
        .unwrap_or(" [q]ueue ✧ [a]dd to playlist ")
}

pub fn create_standard_table<'a>(
    rows: Vec<Row<'a>>,
    title: Line<'static>,
    state: &UiState,
) -> Table<'a> {
    let mode = state.get_mode();
    let theme = state.get_theme(&Pane::TrackList);

    let header = get_header(state, &TableSort::Title);
    let widths = get_widths(mode);
    let keymaps = match state.get_pane() {
        Pane::TrackList => get_keymaps(mode),
        _ => "",
    };

    let block = Block::bordered()
        .title_top(Line::from(title).alignment(Alignment::Center))
        .title_bottom(Line::from(keymaps.fg(theme.text_faded)))
        .title_alignment(Alignment::Center)
        .borders(theme.border_display)
        .border_type(BorderType::Thick)
        .border_style(theme.border)
        .padding(PADDING)
        .bg(theme.bg);

    Table::new(rows, widths)
        .block(block)
        .header(header.fg(theme.text_secondary))
        .column_spacing(COLUMN_SPACING)
        .flex(Flex::Start)
        .highlight_symbol(SELECTOR)
        .highlight_spacing(HighlightSpacing::Always)
        .row_highlight_style(
            Style::new()
                .fg(Color::Black)
                .bg(theme.text_highlighted)
                .italic(),
        )
}

pub fn create_empty_block(theme: &DisplayTheme, title: &str) -> Block<'static> {
    Block::bordered()
        .title_top(format!(" {} ", title))
        .title_alignment(Alignment::Center)
        .borders(theme.border_display)
        .border_type(BorderType::Thick)
        .border_style(theme.border)
        .padding(PADDING)
        .bg(theme.bg)
}

pub struct CellFactory;

impl CellFactory {
    pub fn status_cell(song: &Arc<SimpleSong>, state: &UiState) -> Cell<'static> {
        let theme = state.get_theme(&Pane::TrackList);

        let is_playing = state.get_now_playing().map(|s| s.id) == Some(song.id);

        let is_queued = state
            .playback
            .queue
            .iter()
            .map(|s| s.get_id())
            .collect::<HashSet<_>>()
            .contains(&song.id);
        let is_bulk_selected = state.get_bulk_sel().contains(song);

        Cell::from(if is_playing {
            MUSIC_NOTE.fg(theme.text_secondary)
        } else if is_bulk_selected {
            SELECTED.fg(theme.text_highlighted)
        } else if is_queued {
            QUEUED.fg(theme.text_highlighted)
        } else {
            "".into()
        })
    }

    pub fn title_cell(theme: &DisplayTheme, song: &Arc<SimpleSong>) -> Cell<'static> {
        Cell::from(song.get_title().to_string().fg(theme.text_focused))
    }

    pub fn artist_cell(theme: &DisplayTheme, song: &Arc<SimpleSong>) -> Cell<'static> {
        Cell::from(Line::from(song.get_artist().to_string())).fg(theme.text_focused)
    }

    pub fn filetype_cell(theme: &DisplayTheme, song: &Arc<SimpleSong>) -> Cell<'static> {
        Cell::from(Line::from(format!("{}", song.filetype)).centered()).fg(theme.text_secondary)
    }

    pub fn duration_cell(theme: &DisplayTheme, song: &Arc<SimpleSong>) -> Cell<'static> {
        let duration_str = get_readable_duration(song.get_duration(), DurationStyle::Clean);
        Cell::from(Text::from(duration_str).right_aligned()).fg(theme.text_focused)
    }

    pub fn index_cell(theme: &DisplayTheme, index: usize) -> Cell<'static> {
        Cell::from(format!("{:>2}", index + 1)).fg(theme.text_highlighted)
    }

    pub fn get_track_discs(theme: &DisplayTheme, song: &Arc<SimpleSong>) -> Cell<'static> {
        let track_no = Span::from(match song.track_no {
            Some(t) => format!("{t:>2}"),
            None => format!("{x:>2}", x = "󰇘"),
        })
        .fg(theme.text_highlighted);

        let disc_no = Span::from(match song.disc_no {
            Some(t) => String::from("ᴰ") + SUPERSCRIPT.get(&t).unwrap_or(&"?"),
            None => "".into(),
        })
        .fg(theme.text_faded);

        Cell::from(Line::from_iter([track_no, " ".into(), disc_no.into()]))
    }
}

static SUPERSCRIPT: LazyLock<HashMap<u32, &str>> = LazyLock::new(|| {
    HashMap::from([
        (0, "⁰"),
        (1, "¹"),
        (2, "²"),
        (3, "³"),
        (4, "⁴"),
        (5, "⁵"),
        (6, "⁶"),
        (7, "⁷"),
        (8, "⁸"),
        (9, "⁹"),
    ])
});

fn get_title(state: &UiState, area: Rect) -> Line<'static> {
    let theme = state.get_theme(&Pane::TrackList);
    let (title, track_count) = match state.get_mode() {
        &Mode::Queue => (
            Span::from("Queue").fg(theme.text_highlighted),
            state.playback.queue.len(),
        ),
        &Mode::Library(LibraryView::Playlists) => {
            if state.playlists.is_empty() {
                return "".into();
            }

            let playlist = match state.get_selected_playlist() {
                Some(p) => p,
                None => return "".into(),
            };

            let formatted_title =
                crate::truncate_at_last_space(&playlist.name, (area.width / 3) as usize);
            (
                Span::from(format!("{}", formatted_title))
                    .fg(theme.text_secondary)
                    .italic(),
                playlist.tracklist.len(),
            )
        }
        _ => (Span::default(), 0),
    };

    Line::from_iter([
        Span::from(DECORATOR).fg(theme.text_focused),
        title,
        Span::from(DECORATOR).fg(theme.text_focused),
        Span::from(format!("[{} Songs] ", track_count)).fg(theme.text_faded),
    ])
}

// .\src\tui\widgets\tracklist\search_results.rs
use crate::{
    domain::SongInfo,
    tui::widgets::tracklist::{create_standard_table, CellFactory},
    ui_state::{Pane, TableSort, UiState},
};
use ratatui::{
    style::Stylize,
    text::Line,
    widgets::{StatefulWidget, *},
};

pub struct StandardTable;
impl StatefulWidget for StandardTable {
    type State = UiState;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let theme = &state.get_theme(&Pane::TrackList);

        let songs = state.legal_songs.as_slice();
        let song_len = songs.len();
        let search_len = state.get_search_len();

        let title = match state.get_mode() {
            _ => match search_len > 1 {
                true => format!(" Search Results: {} Songs ", song_len),
                false => format!(" Total: {} Songs ", song_len),
            },
        };

        let rows = songs
            .iter()
            .map(|song| {
                let symbol = CellFactory::status_cell(song, state);
                let mut title_col = Cell::from(song.get_title()).fg(theme.text_faded);
                let mut artist_col = Cell::from(song.get_artist()).fg(theme.text_faded);
                let mut album_col = Cell::from(song.get_album()).fg(theme.text_faded);
                let mut dur_col = Cell::from(Line::from(song.get_duration_str()).right_aligned())
                    .fg(theme.text_faded);

                match state.get_table_sort() {
                    TableSort::Title => title_col = title_col.fg(theme.text_focused),
                    TableSort::Album => album_col = album_col.fg(theme.text_focused),
                    TableSort::Artist => artist_col = artist_col.fg(theme.text_focused),
                    TableSort::Duration => dur_col = dur_col.fg(theme.text_focused),
                }
                Row::new([symbol, title_col, artist_col, album_col, dur_col])
            })
            .collect::<Vec<Row>>();

        let table = create_standard_table(rows, title.into(), state);

        StatefulWidget::render(table, area, buf, &mut state.display_state.table_pos);
    }
}

// .\src\ui_state\album_sort.rs
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum AlbumSort {
    Artist,
    Title,
    Year,
}

impl ToString for AlbumSort {
    fn to_string(&self) -> String {
        match self {
            AlbumSort::Artist => "Artist".into(),
            AlbumSort::Title => "Title".into(),
            AlbumSort::Year => "Year".into(),
        }
    }
}

impl PartialEq<AlbumSort> for &AlbumSort {
    fn eq(&self, other: &AlbumSort) -> bool {
        std::mem::discriminant(*self) == std::mem::discriminant(other)
    }
}

impl AlbumSort {
    pub fn next(&self) -> AlbumSort {
        match self {
            AlbumSort::Artist => AlbumSort::Title,
            AlbumSort::Title => AlbumSort::Year,
            AlbumSort::Year => AlbumSort::Artist,
        }
    }

    pub fn prev(&self) -> AlbumSort {
        match self {
            AlbumSort::Artist => AlbumSort::Year,
            AlbumSort::Title => AlbumSort::Artist,
            AlbumSort::Year => AlbumSort::Title,
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Artist" => AlbumSort::Artist,
            "Title" => AlbumSort::Title,
            "Year" => AlbumSort::Year,
            _ => AlbumSort::Artist,
        }
    }
}

// .\src\ui_state\display_state.rs
use super::{AlbumSort, LibraryView, Mode, Pane, TableSort, UiState};
use crate::{
    domain::{Album, Playlist, SimpleSong, SongInfo},
    key_handler::{Director, MoveDirection},
};
use anyhow::{Result, anyhow};
use indexmap::IndexSet;
use ratatui::widgets::{ListState, TableState};
use std::sync::Arc;

pub struct DisplayState {
    mode: Mode,
    pub pane: Pane,

    table_sort: TableSort,
    pub(super) album_sort: AlbumSort,

    pub sidebar_view: LibraryView,
    pub album_pos: ListState,
    pub playlist_pos: ListState,

    pub table_pos: TableState,
    table_pos_cached: usize,

    pub bulk_select: IndexSet<Arc<SimpleSong>>,
}

impl DisplayState {
    pub fn new() -> Self {
        DisplayState {
            mode: Mode::Library(LibraryView::Albums),
            pane: Pane::TrackList,

            table_sort: TableSort::Title,
            album_sort: AlbumSort::Artist,

            sidebar_view: LibraryView::Albums,
            album_pos: ListState::default().with_selected(Some(0)),
            playlist_pos: ListState::default().with_selected(Some(0)),

            table_pos: TableState::default().with_selected(0),
            table_pos_cached: 0,

            bulk_select: IndexSet::default(),
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

            Mode::Library(view) => {
                self.display_state.sidebar_view = view;
                self.display_state.mode = Mode::Library(view);
                self.display_state.pane = Pane::SideBar;

                // Ensure we have a valid selection for the view we're entering
                match view {
                    LibraryView::Albums => {
                        if self.albums.is_empty() {
                            self.display_state.album_pos.select(None);
                        } else if self.display_state.album_pos.selected().is_none() {
                            self.display_state.album_pos.select(Some(0));
                        }
                    }
                    LibraryView::Playlists => {
                        if self.playlists.is_empty() {
                            self.display_state.playlist_pos.select(None);
                        } else if self.display_state.playlist_pos.selected().is_none() {
                            self.display_state.playlist_pos.select(Some(0));
                        }
                    }
                }

                *self.display_state.table_pos.offset_mut() = 0;
                self.set_legal_songs();
            }
            Mode::Queue => {
                if !self.queue_is_empty() {
                    *self.display_state.table_pos.offset_mut() = 0;
                    self.display_state.mode = Mode::Queue;
                    self.display_state.pane = Pane::TrackList;
                    self.set_legal_songs()
                } else {
                    self.set_error(anyhow!("Queue is empty!"));
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
                let song_ids = self
                    .playback
                    .history
                    .make_contiguous()
                    .iter()
                    .map(|s| s.id)
                    .collect::<Vec<_>>();

                let _ = self.save_state();
                let _ = self.db_worker.save_history_to_db(song_ids);

                self.display_state.mode = Mode::QUIT;
            }
        }
    }

    pub fn get_selected_song(&mut self) -> Result<Arc<SimpleSong>> {
        if self.legal_songs.is_empty() {
            self.display_state.table_pos.select(None);
            return Err(anyhow!("No songs to select!"));
        }

        match self.display_state.mode {
            Mode::Power | Mode::Library(_) | Mode::Search | Mode::Queue => {
                let idx = self
                    .display_state
                    .table_pos
                    .selected()
                    .ok_or_else(|| anyhow!("No song selected!"))?;
                Ok(Arc::clone(&self.legal_songs[idx]))
            }
            Mode::QUIT => unreachable!(),
        }
    }

    pub fn add_to_bulk_select(&mut self) -> Result<()> {
        let song = self.get_selected_song()?;

        match self.display_state.bulk_select.contains(&song) {
            true => self.display_state.bulk_select.swap_remove(&song),
            false => self.display_state.bulk_select.insert(song),
        };

        Ok(())
    }

    pub fn bulk_select_all(&mut self) -> Result<()> {
        if let Mode::Queue | Mode::Library(_) = self.get_mode() {
            let songs = &self.legal_songs;

            match songs
                .iter()
                .all(|s| self.display_state.bulk_select.contains(s))
            {
                true => {
                    songs.iter().for_each(|s| {
                        self.display_state.bulk_select.swap_remove(s);
                    });
                }
                false => {
                    songs.iter().for_each(|s| {
                        self.display_state.bulk_select.insert(Arc::clone(&s));
                    });
                }
            }
        }
        Ok(())
    }

    pub fn get_selected_album(&self) -> Option<&Album> {
        self.display_state
            .album_pos
            .selected()
            .and_then(|idx| self.albums.get(idx))
    }

    pub fn get_selected_playlist(&self) -> Option<&Playlist> {
        self.display_state
            .playlist_pos
            .selected()
            .and_then(|idx| self.playlists.get(idx))
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

    pub(crate) fn shift_position(&mut self, direction: MoveDirection) -> Result<()> {
        match self.get_mode() {
            Mode::Queue => {
                let Some(display_idx) = self.display_state.table_pos.selected() else {
                    return Ok(());
                };

                match direction {
                    MoveDirection::Up => {
                        if display_idx > 0 {
                            self.playback.queue.swap(display_idx, display_idx - 1);
                            self.scroll(Director::Up(1));
                        }
                    }
                    MoveDirection::Down => {
                        if display_idx < self.playback.queue.len() - 1 {
                            self.playback.queue.swap(display_idx, display_idx + 1);
                            self.scroll(Director::Down(1));
                        }
                    }
                }
            }

            Mode::Library(LibraryView::Playlists) => {
                let Some(playlist_idx) = self.display_state.playlist_pos.selected() else {
                    return Ok(());
                };

                let Some(song_idx) = self.display_state.table_pos.selected() else {
                    return Ok(());
                };

                let playlist = &mut self.playlists[playlist_idx];

                match direction {
                    MoveDirection::Up => {
                        if song_idx > 0 && playlist.tracklist.len() > 1 {
                            let ps_id1 = playlist.tracklist[song_idx].id;
                            let ps_id2 = playlist.tracklist[song_idx - 1].id;

                            self.db_worker.swap_position(ps_id1, ps_id2, playlist.id)?;
                            playlist.tracklist.swap(song_idx, song_idx - 1);
                            self.scroll(Director::Up(1));
                        }
                    }
                    MoveDirection::Down => {
                        if song_idx < playlist.tracklist.len() - 1 {
                            let ps_id1 = playlist.tracklist[song_idx].id;
                            let ps_id2 = playlist.tracklist[song_idx + 1].id;

                            self.db_worker.swap_position(ps_id1, ps_id2, playlist.id)?;
                            playlist.tracklist.swap(song_idx, song_idx + 1);
                            self.scroll(Director::Down(1));
                        }
                    }
                }
            }
            _ => (),
        }
        self.set_legal_songs();

        Ok(())
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

        self.legal_songs = this_album
            .ok_or_else(|| anyhow!("Failed to parse album!"))?
            .tracklist
            .clone();

        // Select song and try to visually center it
        self.display_state.table_pos.select(Some(track_idx));
        *self.display_state.table_pos.offset_mut() = track_idx.checked_sub(20).unwrap_or(0);

        // Select album and try to visually center it
        self.display_state.album_pos.select(Some(album_idx));

        Ok(())
    }

    pub(crate) fn set_legal_songs(&mut self) {
        match &self.display_state.mode {
            Mode::Power => {
                self.legal_songs = self.library.get_all_songs().to_vec();
                self.sort_by_table_column();
            }
            Mode::Library(view) => {
                match view {
                    LibraryView::Albums => {
                        if let Some(idx) = self.display_state.album_pos.selected() {
                            if let Some(album) = self.albums.get(idx) {
                                self.legal_songs = album.tracklist.clone();
                            }
                        }
                    }
                    LibraryView::Playlists => {
                        if let Some(idx) = self.display_state.playlist_pos.selected() {
                            if let Some(playlist) = self.playlists.get(idx) {
                                self.legal_songs = playlist.get_tracks()
                            }
                        } else {
                            self.legal_songs.clear()
                        }
                    }
                }
                *self.display_state.table_pos.offset_mut() = 0;
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
        match self.display_state.pane {
            Pane::SideBar => self.scroll_sidebar(&director),
            Pane::TrackList => match director {
                Director::Top => self.scroll_to_top(),
                Director::Bottom => self.scroll_to_bottom(),
                _ => self.scroll_tracklist(&director),
            },
            _ => (),
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
        let (items_len, state) = match self.display_state.sidebar_view {
            LibraryView::Albums => (self.albums.len(), &mut self.display_state.album_pos),
            LibraryView::Playlists => (self.playlists.len(), &mut self.display_state.playlist_pos),
        };

        if items_len == 0 {
            return;
        }

        let current = state.selected().unwrap_or(0);
        let new_pos = match director {
            Director::Up(x) => (current + items_len - x) % items_len,
            Director::Down(x) => (current + x) % items_len,
            Director::Top => 0,
            Director::Bottom => items_len - 1,
        };

        state.select(Some(new_pos));
        self.set_legal_songs();
    }

    fn scroll_to_top(&mut self) {
        match &self.display_state.pane {
            Pane::TrackList => self.display_state.table_pos.select_first(),
            _ => (),
        }
    }

    fn scroll_to_bottom(&mut self) {
        match self.display_state.pane {
            Pane::TrackList => self.display_state.table_pos.select_last(),
            _ => (),
        }
    }
}

// .\src\ui_state\mod.rs
mod album_sort;
mod display_state;
mod mode;
mod pane;
mod playback;
mod playlist;
mod popup;
mod search_state;
mod settings;
mod table_sort;
mod theme;
mod ui_snapshot;
mod ui_state;

pub use album_sort::AlbumSort;
pub use display_state::DisplayState;
pub use mode::LibraryView;
pub use mode::Mode;
pub use pane::Pane;
pub use playlist::PlaylistAction;
pub use popup::PopupType;
pub use settings::SettingsMode;
pub use table_sort::TableSort;
pub use theme::DisplayTheme;
pub use ui_snapshot::UiSnapshot;
pub use ui_state::UiState;

pub use theme::*;

fn new_textarea(placeholder: &str) -> tui_textarea::TextArea<'static> {
    let mut search = tui_textarea::TextArea::default();
    search.set_cursor_line_style(ratatui::style::Style::default());
    search.set_placeholder_text(format!(" {placeholder}: "));

    search
}

// .\src\ui_state\mode.rs
#[derive(Default, PartialEq, Eq, Clone, Copy)]
pub enum LibraryView {
    #[default]
    Albums,
    Playlists,
}

#[derive(PartialEq, Eq, Clone)]
pub enum Mode {
    Power,
    Library(LibraryView),
    Queue,
    Search,
    QUIT,
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Library(LibraryView::default())
    }
}

impl PartialEq<Mode> for &Mode {
    fn eq(&self, other: &Mode) -> bool {
        std::mem::discriminant(*self) == std::mem::discriminant(other)
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Power => write!(f, "power"),
            Mode::Library(LibraryView::Albums) => write!(f, "library_album"),
            Mode::Library(LibraryView::Playlists) => write!(f, "library_playlist"),
            Mode::Queue => write!(f, "queue"),
            Mode::Search => write!(f, "search"),
            Mode::QUIT => write!(f, "quit"),
        }
    }
}

impl Mode {
    pub fn from_str(s: &str) -> Self {
        match s {
            "power" => Mode::Power,
            "library_album" => Mode::Library(LibraryView::Albums),
            "library_playlist" => Mode::Library(LibraryView::Playlists),
            "queue" => Mode::Queue,
            "search" => Mode::Search,
            "quit" => Mode::QUIT,
            _ => Mode::Library(LibraryView::Albums),
        }
    }
}

// .\src\ui_state\pane.rs
#[derive(Default, PartialEq, Eq, Clone)]
pub enum Pane {
    SideBar,
    Search,
    Popup,
    #[default]
    TrackList,
}

impl PartialEq<Pane> for &Pane {
    fn eq(&self, other: &Pane) -> bool {
        std::mem::discriminant(*self) == std::mem::discriminant(other)
    }
}

impl std::fmt::Display for Pane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Pane::TrackList => write!(f, "tracklist"),
            Pane::SideBar => write!(f, "sidebar"),
            Pane::Search => write!(f, "search"),
            Pane::Popup => write!(f, "temp"),
        }
    }
}

impl Pane {
    pub fn from_str(s: &str) -> Self {
        match s {
            "tracklist" => Pane::TrackList,
            "sidebar" => Pane::SideBar,
            "search" => Pane::Search,
            _ => Pane::TrackList,
        }
    }
}

// .\src\ui_state\playback.rs
use super::{Mode, UiState};
use crate::{
    domain::{QueueSong, SimpleSong, SongDatabase},
    player::{PlaybackState, PlayerState},
    strip_win_prefix,
    ui_state::LibraryView,
};
use anyhow::{Context, Result, anyhow};
use std::{
    collections::{HashSet, VecDeque},
    sync::{Arc, Mutex},
    time::Duration,
};

const HISTORY_CAPACITY: usize = 50;
pub struct PlaybackCoordinator {
    pub queue: VecDeque<Arc<QueueSong>>,
    pub history: VecDeque<Arc<SimpleSong>>,
    pub waveform: Vec<f32>,
    pub player_state: Arc<Mutex<PlayerState>>,
}

impl PlaybackCoordinator {
    pub fn new(player_state: Arc<Mutex<PlayerState>>) -> Self {
        PlaybackCoordinator {
            queue: VecDeque::new(),
            history: VecDeque::new(),
            waveform: Vec::new(),
            player_state,
        }
    }
}

// ===================
//   QUEUE & HISTORY
// =================
impl UiState {
    pub fn queue_is_empty(&self) -> bool {
        self.playback.queue.is_empty()
    }

    pub fn queue_song(&mut self, song: Option<Arc<SimpleSong>>) -> Result<()> {
        match self.bulk_select_empty() {
            true => self.add_to_queue_single(song),
            false => self.add_to_queue_bulk(),
        }?;

        self.set_legal_songs();
        Ok(())
    }

    pub(crate) fn add_to_queue_single(&mut self, song: Option<Arc<SimpleSong>>) -> Result<()> {
        let simple_song = match song {
            Some(s) => s,
            None => self.get_selected_song()?,
        };

        let queue_song = self.make_playable_song(&simple_song)?;
        self.playback.queue.push_back(queue_song);
        Ok(())
    }

    pub fn add_to_queue_bulk(&mut self) -> Result<()> {
        let songs;

        if !self.get_bulk_sel().is_empty() {
            songs = self
                .display_state
                .bulk_select
                .clone()
                .into_iter()
                .collect::<Vec<Arc<SimpleSong>>>();
            self.clear_bulk_sel();
        } else {
            songs = match self.get_mode() {
                Mode::Library(LibraryView::Albums) => {
                    let album_idx = self
                        .display_state
                        .album_pos
                        .selected()
                        .ok_or_else(|| anyhow!("Illegal album selection!"))?;

                    self.albums[album_idx].tracklist.clone()
                }
                Mode::Library(LibraryView::Playlists) => {
                    let playlist_idx = self
                        .display_state
                        .playlist_pos
                        .selected()
                        .ok_or_else(|| anyhow!("Illegal playlist selection!"))?;

                    self.playlists[playlist_idx].get_tracks()
                }
                _ => return Ok(()),
            };
        }

        for song in songs {
            self.add_to_queue_single(Some(song))?;
        }
        Ok(())
    }

    pub(crate) fn add_to_history(&mut self, song: Arc<SimpleSong>) {
        if let Some(last) = self.playback.history.front() {
            if last.id == song.id {
                return;
            }
        }

        self.playback.history.push_front(song);
        while self.playback.history.len() > HISTORY_CAPACITY {
            self.playback.history.pop_back();
        }
    }

    pub(crate) fn load_history(&mut self) {
        let song_map = self.library.get_songs_map().to_owned();
        self.playback.history = self.db_worker.import_history(song_map).unwrap_or_default();
    }

    pub fn peek_queue(&self) -> Option<&Arc<SimpleSong>> {
        self.playback.queue.front().map(|q| &q.meta)
    }

    pub fn get_prev_song(&mut self) -> Option<Arc<SimpleSong>> {
        match self.get_now_playing() {
            Some(_) => self.playback.history.remove(1),
            None => self.playback.history.remove(0),
        }
    }

    pub fn remove_song(&mut self) -> Result<()> {
        match self.bulk_select_empty() {
            true => self.remove_song_single()?,
            false => self.remove_song_bulk()?,
        }

        self.set_legal_songs();
        Ok(())
    }

    pub fn remove_song_single(&mut self) -> Result<()> {
        match *self.get_mode() {
            Mode::Library(LibraryView::Playlists) => {
                let song_idx = self
                    .display_state
                    .table_pos
                    .selected()
                    .ok_or_else(|| anyhow!("No song selected"))?;

                let playlist_id = self
                    .get_selected_playlist()
                    .ok_or_else(|| anyhow!("No playlist selected"))?
                    .id;

                let playlist = self
                    .playlists
                    .iter_mut()
                    .find(|p| p.id == playlist_id)
                    .ok_or_else(|| anyhow!("Playlist not found"))?;

                let ps_id = playlist
                    .tracklist
                    .get(song_idx)
                    .ok_or_else(|| anyhow!("Invalid song selection"))?
                    .id;

                self.db_worker.remove_from_playlist(vec![ps_id])?;

                playlist.tracklist.remove(song_idx);
            }
            Mode::Queue => {
                self.display_state
                    .table_pos
                    .selected()
                    .and_then(|idx| self.playback.queue.remove(idx));
            }
            _ => (),
        };
        Ok(())
    }

    pub fn remove_song_bulk(&mut self) -> Result<()> {
        match *self.get_mode() {
            Mode::Library(LibraryView::Playlists) => {
                let playlist_id = self
                    .get_selected_playlist()
                    .ok_or_else(|| anyhow!("No song selected"))?
                    .id;

                let removal_ids = self
                    .get_bulk_sel()
                    .iter()
                    .map(|s| s.id)
                    .collect::<HashSet<_>>();

                let ps_ids_to_remove = {
                    let playlist = self
                        .playlists
                        .iter_mut()
                        .find(|p| p.id == playlist_id)
                        .ok_or_else(|| anyhow!("Playlist not found"))?;

                    playlist
                        .tracklist
                        .iter()
                        .filter(|ps| removal_ids.contains(&ps.song.id))
                        .map(|ps| ps.id)
                        .collect::<Vec<_>>()
                };

                self.db_worker.remove_from_playlist(ps_ids_to_remove)?;

                let playlist = self
                    .playlists
                    .iter_mut()
                    .find(|p| p.id == playlist_id)
                    .ok_or_else(|| anyhow!("Playlist not found"))?;

                playlist
                    .tracklist
                    .retain(|playlist_song| !removal_ids.contains(&playlist_song.song.id));
            }
            Mode::Queue => {
                let removal_ids = self
                    .get_bulk_sel()
                    .iter()
                    .map(|s| s.id)
                    .collect::<HashSet<_>>();

                self.playback
                    .queue
                    .retain(|qs| !removal_ids.contains(&qs.meta.id));
            }
            _ => (),
        }

        self.clear_bulk_sel();
        Ok(())
    }
}

// ===============
//   PlayerState
// =============
impl UiState {
    pub fn update_player_state(&mut self, player_state: Arc<Mutex<PlayerState>>) {
        self.playback.player_state = player_state;
        self.check_player_error();
    }

    pub(crate) fn is_paused(&self) -> bool {
        let state = self.playback.player_state.lock().unwrap();
        state.state == PlaybackState::Paused
    }

    pub fn get_now_playing(&self) -> Option<Arc<SimpleSong>> {
        let state = self.playback.player_state.lock().unwrap();
        state.now_playing.clone()
    }

    pub fn get_playback_elapsed(&self) -> Duration {
        let state = self.playback.player_state.lock().unwrap();
        state.elapsed
    }

    pub fn is_not_playing(&self) -> bool {
        let state = self.playback.player_state.lock().unwrap();
        state.state == PlaybackState::Stopped
    }

    pub fn make_playable_song(&mut self, song: &Arc<SimpleSong>) -> Result<Arc<QueueSong>> {
        let path = song.get_path()?;

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

// ============
//   WAVEFORM
// ==========
impl UiState {
    pub fn get_waveform_visual(&self) -> &[f32] {
        self.playback.waveform.as_slice()
    }

    pub fn set_waveform(&mut self, wf: Vec<f32>) {
        self.playback.waveform = wf
    }

    pub fn clear_waveform(&mut self) {
        self.playback.waveform.clear();
    }

    fn check_player_error(&mut self) {
        let error = self
            .playback
            .player_state
            .lock()
            .unwrap()
            .player_error
            .take();

        if let Some(e) = error {
            self.set_error(e);
        }
    }
}

// .\src\ui_state\playlist.rs
use crate::{
    domain::{Playlist, PlaylistSong},
    ui_state::{LibraryView, PopupType, UiState},
};
use anyhow::{Result, anyhow};

#[derive(PartialEq, Clone)]
pub enum PlaylistAction {
    Create,
    AddSong,
    Delete,
    Rename,
}

impl UiState {
    pub fn get_playlists(&mut self) -> Result<()> {
        let playlist_db = self.db_worker.build_playlists()?;
        let songs_map = self.library.get_songs_map();

        self.playlists = playlist_db
            .iter()
            .map(|((id, name), track_ids)| {
                let tracklist = track_ids
                    .iter()
                    .filter_map(|&s_id| {
                        let ps_id = s_id.0;
                        let simple_song = songs_map.get(&s_id.1)?.clone();

                        Some(PlaylistSong {
                            id: ps_id,
                            song: simple_song,
                        })
                    })
                    .collect();

                Playlist {
                    id: *id,
                    name: name.to_string(),
                    tracklist,
                }
            })
            .collect();

        Ok(())
    }

    pub fn create_playlist_popup(&mut self) {
        if self.get_sidebar_view() == &LibraryView::Playlists {
            self.show_popup(PopupType::Playlist(PlaylistAction::Create));
        }
    }

    pub fn create_playlist(&mut self) -> Result<()> {
        let name = self.get_popup_string();

        if name.is_empty() {
            return Err(anyhow!("Playlist name cannot be empty!"));
        }

        if self
            .playlists
            .iter()
            .any(|p| p.name.to_lowercase() == name.to_lowercase())
        {
            return Err(anyhow!("Playlist name already exists!"));
        }

        self.db_worker.create_playlist(name)?;

        self.get_playlists()?;

        if self.display_state.playlist_pos.selected() == None {
            self.display_state.playlist_pos.select_first();
        }

        self.close_popup();
        Ok(())
    }

    pub fn rename_playlist_popup(&mut self) {
        if self.get_selected_playlist().is_some() {
            self.show_popup(PopupType::Playlist(PlaylistAction::Rename));
        }
    }

    pub fn rename_playlist(&mut self) -> Result<()> {
        let playlist = self
            .get_selected_playlist()
            .ok_or_else(|| anyhow!("No playlist selected!"))?;

        let new_name = self.get_popup_string();

        if new_name.is_empty() {
            return Err(anyhow!("Playlist name cannot be empty!"));
        }

        if self
            .playlists
            .iter()
            .filter(|p| p.id != playlist.id)
            .any(|p| p.name.to_lowercase() == new_name.to_lowercase())
        {
            return Err(anyhow!("Playlist name already exists!"));
        }

        self.db_worker.rename_playlist(playlist.id, new_name)?;

        self.get_playlists()?;
        self.display_state.playlist_pos.select_first();
        self.close_popup();
        Ok(())
    }

    pub fn delete_playlist_popup(&mut self) {
        if self.get_selected_playlist().is_some() {
            self.show_popup(PopupType::Playlist(PlaylistAction::Delete))
        }
    }

    pub fn delete_playlist(&mut self) -> Result<()> {
        let current_playlist = self.display_state.playlist_pos.selected();

        if let Some(idx) = current_playlist {
            let playlist_id = self.playlists[idx].id;

            self.db_worker.delete_playlist(playlist_id)?;

            self.get_playlists()?;
            self.set_legal_songs();
        }

        self.close_popup();

        Ok(())
    }

    pub fn add_to_playlist_popup(&mut self) {
        self.popup.selection.select_first();
        self.show_popup(super::PopupType::Playlist(PlaylistAction::AddSong));
    }

    pub fn add_to_playlist(&mut self) -> Result<()> {
        match self.popup.selection.selected() {
            Some(playlist_idx) => {
                let playlist_id = self.playlists.get(playlist_idx).unwrap().id;
                match self.get_bulk_sel().is_empty() {
                    true => {
                        let song_id = self.get_selected_song()?.id;

                        self.db_worker.add_to_playlist(song_id, playlist_id)?;
                    }
                    false => {
                        let song_ids = self.get_bulk_sel().iter().map(|s| s.id).collect::<Vec<_>>();

                        self.db_worker.add_to_playlist_bulk(song_ids, playlist_id)?;
                        self.clear_bulk_sel();
                    }
                }
                self.close_popup()
            }
            None => return Err(anyhow!("Could not add to playlist")),
        };

        self.get_playlists()?;

        Ok(())
    }
}

// .\src\ui_state\popup.rs
use ratatui::{crossterm::event::KeyEvent, widgets::ListState};
use tui_textarea::TextArea;

use crate::{
    get_random_playlist_idea,
    ui_state::{Pane, SettingsMode, UiState, new_textarea, playlist::PlaylistAction},
};

#[derive(PartialEq, Clone)]
pub enum PopupType {
    None,
    Error(String),
    Settings(SettingsMode),
    Playlist(PlaylistAction),
}

pub struct PopupState {
    pub current: PopupType,
    pub input: TextArea<'static>,
    pub selection: ListState,
    pub cached: Pane,
}

impl PopupState {
    pub(crate) fn new() -> PopupState {
        PopupState {
            current: PopupType::None,
            input: new_textarea(""),
            selection: ListState::default(),
            cached: Pane::Popup,
        }
    }

    fn open(&mut self, popup: PopupType) {
        match &popup {
            PopupType::Playlist(PlaylistAction::Rename)
            | PopupType::Playlist(PlaylistAction::Create) => {
                let placeholder = get_random_playlist_idea();

                self.input.set_placeholder_text(format!(" {placeholder} "));
                self.input.select_all();
                self.input.cut();
            }
            PopupType::Settings(SettingsMode::ViewRoots) => {
                self.input.select_all();
                self.input.cut();
            }
            PopupType::Settings(SettingsMode::AddRoot) => {
                self.input
                    .set_placeholder_text(" Enter path to directory: ");
                self.input.select_all();
                self.input.cut();
            }

            _ => (),
        }
        self.current = popup
    }

    pub fn is_open(&self) -> bool {
        self.current != PopupType::None
    }

    fn close(&mut self) -> Pane {
        self.current = PopupType::None;
        self.input.select_all();
        self.input.cut();

        self.cached.clone()
    }

    fn set_cached_pane(&mut self, pane: Pane) {
        self.cached = pane
    }
}

impl UiState {
    pub fn show_popup(&mut self, popup: PopupType) {
        self.popup.open(popup);
        if self.popup.cached == Pane::Popup {
            let current_pane = self.get_pane().clone();
            self.popup.set_cached_pane(current_pane);
            self.set_pane(Pane::Popup);
        }
    }

    pub fn get_popup_string(&self) -> String {
        self.popup.input.lines()[0].trim().to_string()
    }

    pub fn close_popup(&mut self) {
        let pane = self.popup.close();
        self.popup.cached = Pane::Popup;
        self.set_pane(pane);
    }

    pub fn process_popup_input(&mut self, key: &KeyEvent) {
        self.popup.input.input(*key);
    }
}

// .\src\ui_state\search_state.rs
use super::{Pane, UiState, new_textarea};
use crate::domain::{SimpleSong, SongInfo};
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use ratatui::crossterm::event::KeyEvent;
use std::sync::Arc;
use tui_textarea::TextArea;

const MATCH_THRESHOLD: i64 = 70;

pub(super) struct SearchState {
    pub input: TextArea<'static>,
    matcher: SkimMatcherV2,
}

impl SearchState {
    pub fn new() -> Self {
        SearchState {
            input: new_textarea("Enter search query"),
            matcher: SkimMatcherV2::default(),
        }
    }
}

impl UiState {
    // Algorithm looks at the title, artist, and album fields
    // and scores each attribute while applying a heavier
    // weight to the title field and returns the highest score.
    // Assuming the score is higher than the threshold, the
    // result is valid. Results are ordered by score.
    pub(crate) fn filter_songs_by_search(&mut self) {
        let query = self.read_search().to_lowercase();

        let mut scored_songs: Vec<(Arc<SimpleSong>, i64)> = self
            .library
            .get_all_songs()
            .iter()
            .filter_map(|song| {
                let title_score = self
                    .search
                    .matcher
                    .fuzzy_match(&song.get_title().to_lowercase(), &query)
                    .unwrap_or(0);

                let artist_score = self
                    .search
                    .matcher
                    .fuzzy_match(&song.get_artist().to_lowercase(), &query)
                    .unwrap_or(0);

                let album_score = self
                    .search
                    .matcher
                    .fuzzy_match(&song.get_album().to_lowercase(), &query)
                    .unwrap_or(0);

                // Apply height weight to title.
                let weighted_score = [(title_score * 2) + artist_score + album_score];
                let best_score = weighted_score.iter().max().copied().unwrap_or(0);

                if best_score > MATCH_THRESHOLD {
                    Some((Arc::clone(&song), best_score))
                } else {
                    None
                }
            })
            .collect();

        scored_songs.sort_by(|a, b| b.1.cmp(&a.1));
        self.legal_songs = scored_songs.into_iter().map(|i| i.0).collect();
    }

    pub fn get_search_widget(&mut self) -> &mut TextArea<'static> {
        &mut self.search.input
    }

    pub fn get_search_len(&self) -> usize {
        self.search.input.lines()[0].len()
    }

    pub fn send_search(&mut self) {
        match !self.legal_songs.is_empty() {
            true => self.set_pane(Pane::TrackList),
            false => self.soft_reset(),
        }
    }

    pub fn process_search(&mut self, k: KeyEvent) {
        self.search.input.input(k);
        self.set_legal_songs();
        match self.legal_songs.is_empty() {
            true => self.display_state.table_pos.select(None),
            false => self.display_state.table_pos.select(Some(0)),
        }
    }

    pub fn read_search(&self) -> &str {
        &self.search.input.lines()[0]
    }
}

// .\src\ui_state\settings\mod.rs
mod root_mgmt;

#[derive(Default, PartialEq, Clone)]
pub enum SettingsMode {
    #[default]
    ViewRoots,
    AddRoot,
    RemoveRoot,
}

// .\src\ui_state\settings\root_mgmt.rs
use crate::{
    Library,
    app_core::Concertus,
    ui_state::{PopupType, SettingsMode, UiState},
};
use anyhow::{Result, anyhow};
use std::sync::Arc;

impl UiState {
    pub fn get_settings_mode(&self) -> Option<&SettingsMode> {
        match &self.popup.current {
            PopupType::Settings(mode) => Some(mode),
            _ => None,
        }
    }

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
        let mut lib = Library::init();
        lib.add_root(path)?;
        lib.build_library()?;

        self.library = Arc::new(lib);

        Ok(())
    }

    pub fn remove_root(&mut self) -> Result<()> {
        if let Some(selected) = self.popup.selection.selected() {
            let roots = self.get_roots();
            if selected >= roots.len() {
                return Err(anyhow!("Invalid root index!"));
            }

            let mut lib = Library::init();

            let bad_root = &roots[selected];
            lib.delete_root(&bad_root)?;
        }

        Ok(())
    }

    pub fn enter_settings(&mut self) {
        if !self.get_roots().is_empty() {
            self.popup.selection.select(Some(0));
        }

        self.show_popup(PopupType::Settings(SettingsMode::ViewRoots));
    }
}

impl Concertus {
    pub(crate) fn settings_remove_root(&mut self) {
        if !self.ui.get_roots().is_empty() {
            self.ui
                .show_popup(PopupType::Settings(SettingsMode::RemoveRoot));
        }
    }

    pub(crate) fn activate_settings(&mut self) {
        match self.ui.get_roots().is_empty() {
            true => self.ui.popup.selection.select(None),
            false => self.ui.popup.selection.select(Some(0)),
        }
        self.ui
            .show_popup(PopupType::Settings(SettingsMode::ViewRoots))
    }

    pub(crate) fn popup_scroll_up(&mut self) {
        let list_len = match self.ui.popup.current {
            PopupType::Settings(_) => self.ui.get_roots().len(),
            PopupType::Playlist(_) => self.ui.playlists.len(),
            _ => return,
        };

        if list_len > 0 {
            let current = self.ui.popup.selection.selected().unwrap_or(0);
            let new_selection = if current > 0 {
                current - 1
            } else {
                list_len - 1 // Wrap to bottom
            };
            self.ui.popup.selection.select(Some(new_selection));
        }
    }

    pub(crate) fn popup_scroll_down(&mut self) {
        let list_len = match self.ui.popup.current {
            PopupType::Settings(_) => self.ui.get_roots().len(),
            PopupType::Playlist(_) => self.ui.playlists.len(),
            _ => return,
        };

        if list_len > 0 {
            let current = self.ui.popup.selection.selected().unwrap_or(0);
            let new_selection = (current + 1) % list_len; // Wrap to top
            self.ui.popup.selection.select(Some(new_selection));
        }
    }

    pub(crate) fn settings_add_root(&mut self) {
        self.ui
            .show_popup(PopupType::Settings(SettingsMode::AddRoot));
    }

    pub(crate) fn settings_root_confirm(&mut self) -> anyhow::Result<()> {
        match self.ui.popup.current {
            PopupType::Settings(SettingsMode::AddRoot) => {
                let path = self.ui.get_popup_string();
                if !path.is_empty() {
                    match self.ui.add_root(&path) {
                        Err(e) => self.ui.set_error(e),
                        Ok(_) => {
                            self.update_library()?;
                            self.ui.close_popup();
                        }
                    }
                }
            }
            PopupType::Settings(SettingsMode::RemoveRoot) => {
                if let Err(e) = self.ui.remove_root() {
                    self.ui.set_error(e);
                } else {
                    self.ui
                        .show_popup(PopupType::Settings(SettingsMode::ViewRoots));
                    self.ui.popup.selection.select(Some(0));
                    self.update_library()?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

// .\src\ui_state\table_sort.rs
#[derive(PartialEq)]
pub enum TableSort {
    Title,
    Artist,
    Album,
    Duration,
}

impl ToString for TableSort {
    fn to_string(&self) -> String {
        match self {
            TableSort::Title => "Title".into(),
            TableSort::Artist => "Artist".into(),
            TableSort::Album => "Album".into(),
            TableSort::Duration => "Duration".into(),
        }
    }
}

impl TableSort {
    pub fn next(&self) -> Self {
        match self {
            TableSort::Title => TableSort::Artist,
            TableSort::Artist => TableSort::Album,
            TableSort::Album => TableSort::Duration,
            TableSort::Duration => TableSort::Title,
        }
    }
    pub fn prev(&self) -> Self {
        match self {
            TableSort::Title => TableSort::Duration,
            TableSort::Artist => TableSort::Title,
            TableSort::Album => TableSort::Artist,
            TableSort::Duration => TableSort::Album,
        }
    }
}

// .\src\ui_state\theme.rs
use ratatui::{style::Color, widgets::Borders};

use crate::ui_state::{Pane, UiState};

const DARK_WHITE: Color = Color::Rgb(210, 210, 210);
const MID_GRAY: Color = Color::Rgb(100, 100, 100);
const DARK_GRAY: Color = Color::Rgb(25, 25, 25);
const DARK_GRAY_FADED: Color = Color::Rgb(10, 10, 10);
pub const GOOD_RED: Color = Color::Rgb(255, 70, 70);
pub const GOOD_RED_DARK: Color = Color::Rgb(180, 50, 50);
pub const GOLD: Color = Color::Rgb(220, 220, 100);
pub const GOLD_FADED: Color = Color::Rgb(130, 130, 60);

pub struct DisplayTheme {
    pub bg: Color,
    pub border: Color,
    pub border_display: Borders,
    pub text_focused: Color,
    pub text_secondary: Color,
    pub text_faded: Color,
    pub text_highlighted: Color,
}

pub(crate) struct Theme {
    pub bg_focused: Color,
    pub bg_unfocused: Color,
    pub border_focused: Color,
    pub border_unfocused: Color,
    pub text_focused: Color,
    pub text_secondary: Color,
    pub text_secondary_u: Color,
    pub text_unfocused: Color,
    pub text_highlighted: Color,
    pub text_highlighted_u: Color,
}

impl Theme {
    pub fn set_generic_theme() -> Theme {
        Theme {
            bg_focused: DARK_GRAY,
            bg_unfocused: DARK_GRAY_FADED,

            text_focused: DARK_WHITE,
            text_unfocused: MID_GRAY,
            text_secondary: GOOD_RED,
            text_secondary_u: GOOD_RED_DARK,
            text_highlighted: GOLD,
            text_highlighted_u: GOLD_FADED,

            border_focused: GOLD,
            border_unfocused: Color::Rgb(50, 50, 50),
        }
    }
}

impl UiState {
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
}

// .\src\ui_state\ui_snapshot.rs
use anyhow::Result;

use super::{AlbumSort, Mode, Pane, UiState};

#[derive(Default)]
pub struct UiSnapshot {
    pub mode: String,
    pub pane: String,
    pub album_sort: String,
    pub album_selection: Option<usize>,
    pub playlist_selection: Option<usize>,
    pub song_selection: Option<usize>,
}

impl UiSnapshot {
    pub fn to_pairs(&self) -> Vec<(&'static str, String)> {
        let mut pairs = vec![
            ("ui_mode", self.mode.clone()),
            ("ui_pane", self.pane.clone()),
            ("ui_album_sort", self.album_sort.clone()),
        ];

        if let Some(pos) = self.album_selection {
            pairs.push(("ui_album_pos", pos.to_string()));
        }

        if let Some(pos) = self.playlist_selection {
            pairs.push(("ui_playlist_pos", pos.to_string()));
        }

        if let Some(pos) = self.song_selection {
            pairs.push(("ui_song_pos", pos.to_string()));
        }

        pairs
    }

    pub fn from_values(values: Vec<(String, String)>) -> Self {
        let mut snapshot = UiSnapshot::default();

        for (key, value) in values {
            match key.as_str() {
                "ui_mode" => snapshot.mode = value,
                "ui_pane" => snapshot.pane = value,
                "ui_album_sort" => snapshot.album_sort = value,
                "ui_album_pos" => snapshot.album_selection = value.parse().ok(),
                "ui_playlist_pos" => snapshot.playlist_selection = value.parse().ok(),
                "ui_song_pos" => snapshot.song_selection = value.parse().ok(),
                _ => {}
            }
        }

        snapshot
    }
}

impl UiState {
    pub fn create_snapshot(&self) -> UiSnapshot {
        let orig_pane = self.get_pane();
        let pane = match orig_pane {
            Pane::Popup => &self.popup.cached,
            _ => orig_pane,
        };

        UiSnapshot {
            mode: self.get_mode().to_string(),
            pane: pane.to_string(),
            album_sort: self.display_state.album_sort.to_string(),
            album_selection: self.display_state.album_pos.selected(),
            playlist_selection: self.display_state.playlist_pos.selected(),
            song_selection: self.display_state.table_pos.selected(),
        }
    }

    pub fn save_state(&self) -> Result<()> {
        let snapshot = self.create_snapshot();
        self.db_worker.save_ui_snapshot(snapshot)?;
        Ok(())
    }

    pub fn restore_state(&mut self) -> Result<()> {
        // The order of these function calls is particularly important
        if let Some(snapshot) = self.db_worker.load_ui_snapshot()? {
            self.display_state.album_sort = AlbumSort::from_str(&snapshot.album_sort);

            self.sort_albums();

            if let Some(pos) = snapshot.album_selection {
                if pos < self.albums.len() {
                    self.display_state.album_pos.select(Some(pos));
                }
            }

            if let Some(pos) = snapshot.playlist_selection {
                if pos < self.playlists.len() {
                    self.display_state.playlist_pos.select(Some(pos));
                }
            }

            self.set_mode(Mode::from_str(&snapshot.mode));
            self.set_pane(Pane::from_str(&snapshot.pane));

            if let Some(pos) = snapshot.song_selection {
                if pos < self.legal_songs.len() {
                    self.display_state.table_pos.select(Some(pos));
                }
            }
        }

        Ok(())
    }
}

// .\src\ui_state\ui_state.rs
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

