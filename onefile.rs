// Project: concertus (v0.1.0)

// .\src\app_core\app.rs
use crate::{
    domain::{generate_waveform, QueueSong, SongInfo},
    key_handler::{self, Action},
    overwrite_line,
    player::PlayerController,
    tui,
    ui_state::{Mode, Pane, SettingsMode, UiState},
    Database, Library,
};
use anyhow::Result;
use ratatui::crossterm::event::{Event, KeyEventKind};
use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

pub struct Concertus {
    _initializer: Instant,
    db: Arc<Mutex<Database>>,
    library: Arc<Library>,
    pub(crate) ui: UiState,
    player: PlayerController,
    waveform_rec: Option<mpsc::Receiver<Vec<f32>>>,
    requires_setup: bool,
}

impl Concertus {
    pub fn new() -> Self {
        let db = Database::open().expect("Could not create database!");
        let db = Arc::new(Mutex::new(db));
        let lib = Library::init(Arc::clone(&db));
        let lib = Arc::new(lib);
        let lib_clone = Arc::clone(&lib);

        let shared_state = Arc::new(Mutex::new(crate::player::PlayerState::default()));
        let shared_state_clone = Arc::clone(&shared_state);

        let appstate = Concertus {
            _initializer: Instant::now(),
            db,
            library: lib,
            player: PlayerController::new(),
            ui: UiState::new(lib_clone, shared_state_clone),
            waveform_rec: None,
            requires_setup: true,
        };

        appstate
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let mut terminal = ratatui::init();
        terminal.clear()?;

        self.preload_lib();
        self.initialize_ui();

        match self.requires_setup {
            true => {
                self.ui.set_pane(Pane::Popup);
                self.ui.settings.settings_mode = SettingsMode::AddRoot;
            }
            false => (),
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
        let lib_db = Arc::clone(&self.db);
        let mut updated_lib = Library::init(lib_db);

        if !updated_lib.roots.is_empty() {
            self.requires_setup = false
        };

        // TODO: MAKE THIS OPTIONAL
        // updated_lib.update_db().unwrap();
        updated_lib.build_library().unwrap();

        self.library = Arc::new(updated_lib);
        self.ui.sync_library(Arc::clone(&self.library));
    }

    pub fn initialize_ui(&mut self) {
        self.ui.soft_reset();
        self.ui.sync_library(Arc::clone(&self.library));
        self.ui.load_history();
        let _ = self.ui.restore_state();
    }
}

impl Concertus {
    #[rustfmt::skip]
    fn handle_action(&mut self, action: Action) -> Result<()> {
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
            Action::ToggleAlbumSort(next) => self.ui.toggle_album_sort(next),

            // Search Related
            Action::UpdateSearch(k) => self.ui.process_search(k),
            Action::SendSearch      => self.ui.send_search(),

            // Queue
            Action::QueueSong       => self.ui.queue_song(None)?,
            Action::QueueAlbum      => self.ui.queue_album()?,
            Action::RemoveFromQueue => self.ui.remove_from_queue()?,

            // Ops
            Action::SoftReset       => self.ui.soft_reset(),
            Action::UpdateLibrary   => self.update_library()?,
            Action::QUIT            => self.ui.set_mode(Mode::QUIT),

            Action::ViewSettings    => self.activate_settings(),
            Action::SettingsUp      => self.settings_scroll_up(),
            Action::SettingsDown    => self.settings_scroll_down(),
            Action::RootAdd         => self.settings_add_root(),
            Action::RootRemove      => self.settings_remove_root(),
            Action::RootConfirm     => self.settings_root_confirm()?,

            Action::SettingsInput(key) => {
                self.ui.settings.root_input.input(key);
            }
            _ => (),
        }
        Ok(())
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
            return Err(anyhow::anyhow!("File not found: {}", &song.path));
        }

        self.ui.clear_waveform();
        self.waveform_handler(&song)?;
        self.library.update_play_count(&song.meta);
        self.player.play_song(song)?;

        Ok(())
    }

    fn play_selected_song(&mut self) -> Result<()> {
        let song = self.ui.get_selected_song()?;
        let queue_song = self.ui.make_playable_song(&song)?;

        self.ui.add_to_history(Arc::clone(&song));

        self.play_song(queue_song)
    }

    fn play_next(&mut self) -> Result<()> {
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

    fn play_prev(&mut self) -> Result<()> {
        match self.ui.get_prev_song() {
            Some(prev) => {
                if let Some(now_playing) = self.ui.get_now_playing() {
                    let queue_song = self.ui.make_playable_song(&now_playing)?;
                    self.ui.playback.queue.push_front(queue_song);
                }
                let queue_song = self.ui.make_playable_song(&prev)?;
                self.play_song(queue_song)?;
            }
            None => self.ui.set_error(anyhow::anyhow!("End of history!")),
        }

        self.ui.set_legal_songs();
        Ok(())
    }
}

impl Concertus {
    fn waveform_handler(&mut self, song: &QueueSong) -> Result<()> {
        let path_clone = song.path.clone();

        let mut db = self.db.lock().unwrap();
        match db.get_waveform(&song.path) {
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
        if self.ui.get_waveform().is_empty() && self.ui.get_now_playing().is_some() {
            if let Some(rx) = &self.waveform_rec {
                if let Ok(waveform) = rx.try_recv() {
                    let id = self.player.get_now_playing().unwrap().id;
                    let mut db = self.db.lock().unwrap();

                    db.set_waveform(id, &waveform)?;
                    self.ui.set_waveform(waveform);
                    self.waveform_rec = None;
                    return Ok(());
                }
            }
            return Err(anyhow::format_err!("Invalid waveform"));
        }
        Ok(())
    }

    pub(crate) fn update_library(&mut self) -> Result<()> {
        let lib_db = Arc::clone(&self.db);
        let mut updated_lib = Library::init(lib_db);

        let cached = self.ui.display_state.sidebar_pos.selected();
        self.ui.display_state.sidebar_pos.select(None);

        // TODO: Alert user of changes on update
        updated_lib.update_db_by_root()?;
        updated_lib.build_library()?;

        let updated_len = updated_lib.albums.len();

        self.library = Arc::new(updated_lib);
        self.ui.sync_library(Arc::clone(&self.library));

        // Do not index a value out of bounds if current selection
        // will be out of bounds after update

        if updated_len > 0 {
            self.ui
                .display_state
                .sidebar_pos
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
use anyhow::Result;
use queries::*;
use rusqlite::{params, Connection};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs,
    path::PathBuf,
    sync::Arc,
    time::{Duration, UNIX_EPOCH},
};

pub mod queries;
mod tables;

use crate::{
    domain::{LongSong, SimpleSong, SongInfo},
    ui_state::UiSnapshot,
};

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
                    &song.format
                ])
                .unwrap_or_else(|e| {
                    eprintln!("Error inserting song {}: {}", song.title, e);
                    0
                });
            }
        }
        tx.commit()?;

        Ok(())
    }

    pub(crate) fn get_all_songs(&mut self) -> Result<Vec<Arc<SimpleSong>>> {
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
                    format: row.get("format")?,
                };

                Ok(Arc::new(song))
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

    pub fn get_waveform(&mut self, path: &str) -> Result<Vec<f32>> {
        let blob: Vec<u8> = self
            .conn
            .query_row(GET_WAVEFORM, params![path], |row| row.get(0))?;
        Ok(bincode::decode_from_slice(&blob, bincode::config::standard())?.0)
    }

    pub fn set_waveform(&mut self, id: u64, wf: &[f32]) -> Result<()> {
        // let serialized = bincode::serialize(wf)?;
        let serialized = bincode::encode_to_vec(wf, bincode::config::standard())?;

        self.conn
            .execute(INSERT_WAVEFORM, params![id.to_le_bytes(), serialized])?;

        Ok(())
    }

    // ============
    //   HISTORY
    // ============

    pub fn save_history_to_db(&mut self, history: &[Arc<SimpleSong>]) -> Result<()> {
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
            for (idx, song) in history.iter().enumerate() {
                stmt.execute(params![song.id.to_le_bytes(), timestamp - idx as i64])?;
            }
            tx.execute(DELETE_FROM_HISTORY, [])?;
        }
        tx.commit()?;

        Ok(())
    }

    pub fn import_history(
        &mut self,
        songs: &[Arc<SimpleSong>],
    ) -> Result<VecDeque<Arc<SimpleSong>>> {
        let mut history = VecDeque::new();

        let song_map: HashMap<u64, Arc<SimpleSong>> =
            songs.iter().map(|s| (s.id, Arc::clone(s))).collect();

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

    pub(crate) fn update_play_count(&mut self, song: &Arc<SimpleSong>) -> Result<()> {
        let id = song.id.to_le_bytes();
        self.conn.execute(UPDATE_PLAY_COUNT, params![id, 1])?;

        Ok(())
    }

    pub(crate) fn get_path(&mut self, id: u64) -> Result<String> {
        let output = self
            .conn
            .query_row(GET_PATH, [id.to_le_bytes()], |r| r.get(0))?;
        Ok(output)
    }

    pub fn save_session_state(&mut self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(SET_SESSION_STATE, params![key, value])?;
        Ok(())
    }

    pub fn get_session_state(&mut self, key: &str) -> Result<Option<String>> {
        match self.conn.query_row(GET_SESSION_STATE, params![key], |row| {
            row.get::<_, String>(0)
        }) {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn save_ui_snapshot(&mut self, snapshot: &UiSnapshot) -> Result<()> {
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
        let mut stmt = self
            .conn
            .prepare("SELECT key, value FROM session_state WHERE key LIKE 'ui_%'")?;

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

// .\src\database\queries.rs
pub const GET_WAVEFORM: &str = "
    SELECT w.waveform 
    FROM waveforms w
    JOIN songs s on w.song_id = s.id
    WHERE s.path = ?
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

// KEEP AN EYE ON THIS
// MIGHT REVERT TO INSERT OR IGNORE
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

// pub const GET_SONGS: &str = "
//     SELECT
//         s.id,
//         s.title as title,
//         ar.name as artist,
//         al.title as album,
//         art_album.name as album_artist,
//         s.track_no,
//         s.disc_no,
//         s.duration
//     FROM songs s
//     LEFT JOIN artists ar ON ar.id = s.artist_id
//     LEFT JOIN albums al ON al.id = s.album_id;
//     LEFT JOIN artists art_album ON art_album.id = al.artist_id
// ";

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

pub const SET_SESSION_STATE: &str = "
    INSERT OR REPLACE INTO session_state (key, value) VALUES (?, ?)
";

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
        song_id BLOB,
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
        song_id BLOB UNIQUE NOT NULL,
        count INTEGER,
        FOREIGN KEY(song_id) REFERENCES songs(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS session_state(
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
";

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
#[derive(Default, PartialEq, Copy, Clone)]
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
use anyhow::{Context, Result};
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
    pub(crate) format: FileType,
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
            None => {
                return Err(anyhow::format_err!(
                    "Unsuppored extension: {:?}",
                    path.extension()
                ))
            }
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

        song_info.format = format;
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

        if song_info.format == FileType::M4A {
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
mod queue_song;
mod simple_song;
mod waveform;

pub use album::Album;
pub use filetype::FileType;
pub use long_song::LongSong;
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

// .\src\domain\queue_song.rs
use super::{SimpleSong, SongInfo};
use crate::get_readable_duration;
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

// .\src\domain\simple_song.rs
use super::{FileType, SongInfo};
use crate::{get_readable_duration, Database};
use std::{sync::Arc, time::Duration};

#[derive(Default)]
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
    pub(crate) format: FileType,
}

impl SimpleSong {
    pub fn get_path(&self, db: &mut Database) -> anyhow::Result<String> {
        db.get_path(self.id)
    }
}

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

// .\src\domain\waveform.rs
use anyhow::{Context, Result};
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
            // eprintln!("Error generating waveform: {}", e);
            vec![0.3; WF_LEN] // Return a flat line if all fails
        }
    }
}

/// Extract duration from audio file using ffmpeg
fn get_audio_duration<P: AsRef<Path>>(audio_path: P) -> Result<Duration> {
    let audio_path_str = audio_path
        .as_ref()
        .to_str()
        .ok_or_else(|| anyhow::format_err!("Audio path contains invalid Unicode"))?;

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
        return Err(anyhow::format_err!(
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
        Err(e) => {
            eprintln!("Warning: Couldn't determine audio duration: {}", e);
            return Err(anyhow::anyhow!("Could not determine audio length"));
        }
    };

    // Calculate adaptive samples per point based on duration
    let samples_per_point = calculate_adaptive_samples(duration);

    // Get the path as string, with better error handling
    let audio_path_str = audio_path
        .as_ref()
        .to_str()
        .ok_or_else(|| anyhow::format_err!("Audio path contains invalid Unicode"))?;

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
        return Err(anyhow::format_err!(
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
    ui_state::{Mode, Pane, SettingsMode, UiState},
    REFRESH_RATE,
};
use anyhow::Result;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::{collections::HashSet, sync::LazyLock, time::Duration};

static ILLEGAL_CHARS: LazyLock<HashSet<char>> = LazyLock::new(|| HashSet::from(['\'', ';']));
const C: KeyModifiers = KeyModifiers::CONTROL;
const X: KeyModifiers = KeyModifiers::NONE;

const SEEK_SMALL: usize = 5;
const SEEK_LARGE: usize = 30;
const SCROLL_MID: usize = 5;
const SCROLL_XTRA: usize = 50;

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

    // Queue Controls
    QueueSong,
    QueueAlbum,
    RemoveFromQueue,

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

    // Errors, Convenience & Other
    ViewSettings,
    SettingsUp,
    SettingsDown,
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
   
    let _mode = state.get_mode();
    let pane = state.get_pane();

    if let Some(action) = global_commands(&key_event, pane) {
        return Some(action)
    } 

    match pane {
        Pane::TrackList => handle_main_pane(&key_event),
        Pane::Search    => handle_search_pane(&key_event),
        Pane::SideBar   => handle_sidebar_pane(&key_event),
        Pane::Popup     => handle_popup_pane(&key_event, state),
    }
}

fn global_commands(key: &KeyEvent, pane: &Pane) -> Option<Action> {
    let in_search = pane == Pane::Search;
    let in_settings = pane == Pane::Popup;

    // Works on every pane, even search
    match (key.modifiers, key.code) {
        (_, Esc) => Some(Action::SoftReset),
        (C, Char('c')) => Some(Action::QUIT),
        (_, Char('`')) => Some(Action::ViewSettings),
        (C, Char(' ')) => Some(Action::TogglePause),

        // Works on everything except search
        _ if (!in_search && !in_settings) => match (key.modifiers, key.code) {
            // PLAYBACK COMMANDS
            (_, Char(' ')) => Some(Action::TogglePause),
            (C, Char('s')) => Some(Action::Stop),
            (C, Char('n')) => Some(Action::PlayNext),
            (C, Char('p')) => Some(Action::PlayPrev),
            (_, Char('n')) => Some(Action::SeekForward(SEEK_SMALL)),
            (_, Char('N')) => Some(Action::SeekForward(SEEK_LARGE)),
            (_, Char('p')) => Some(Action::SeekBack(SEEK_SMALL)),
            (_, Char('P')) => Some(Action::SeekBack(SEEK_LARGE)),

            // NAVIGATION
            (C, Char('z')) => Some(Action::ChangeMode(Mode::Power)),
            (_, Char('/')) => Some(Action::ChangeMode(Mode::Search)),
            (C, Char('q')) => Some(Action::ChangeMode(Mode::Queue)),

            // SCROLLING
            (_, Char('j')) | (_, Down) => Some(Action::Scroll(Director::Down(1))),
            (_, Char('k')) | (_, Up) => Some(Action::Scroll(Director::Up(1))),
            (_, Char('d')) => Some(Action::Scroll(Director::Down(SCROLL_MID))),
            (X, Char('u')) => Some(Action::Scroll(Director::Up(SCROLL_MID))),
            (_, Char('D')) => Some(Action::Scroll(Director::Down(SCROLL_XTRA))),
            (_, Char('U')) => Some(Action::Scroll(Director::Up(SCROLL_XTRA))),
            (_, Char('g')) => Some(Action::Scroll(Director::Top)),
            (_, Char('G')) => Some(Action::Scroll(Director::Bottom)),

            (C, Char('u')) | (_, F(5)) => Some(Action::UpdateLibrary),

            _ => None,
        },
        _ => None,
    }
}

fn handle_main_pane(key: &KeyEvent) -> Option<Action> {
    match (key.modifiers, key.code) {
        // QUEUEING SONGS
        (_, Char('q')) => Some(Action::QueueSong),
        (_, Char('x')) => Some(Action::RemoveFromQueue),

        (C, Char('a')) => Some(Action::GoToAlbum),

        // SORTING SONGS
        (_, Left) | (_, Char('h')) => Some(Action::SortColumnsPrev),
        (_, Right) | (_, Char('l')) => Some(Action::SortColumnsNext),

        (_, Enter) => Some(Action::Play),
        (_, Tab) => Some(Action::ChangeMode(Mode::Album)),
        _ => None,
    }
}

fn handle_sidebar_pane(key: &KeyEvent) -> Option<Action> {
    match (key.modifiers, key.code) {
        (_, Char('q')) => Some(Action::QueueAlbum),
        (_, Enter) | (_, Tab) => Some(Action::ChangePane(Pane::TrackList)),
        (C, Left) | (C, Char('h')) => Some(Action::ToggleAlbumSort(false)),
        (C, Right) | (C, Char('l')) => Some(Action::ToggleAlbumSort(true)),

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

fn handle_popup_pane(key: &KeyEvent, state: &UiState) -> Option<Action> {
    if let Some(_) = state.get_error() {
        match key.code {
            Char('?') | Char('`') | Enter | Esc => Some(Action::SoftReset),
            _ => None,
        };
    }

    match state.get_settings_mode() {
        SettingsMode::ViewRoots => match key.code {
            Char('a') => Some(Action::RootAdd),
            Char('d') => Some(Action::RootRemove),
            Up | Char('k') => Some(Action::SettingsUp),
            Down | Char('j') => Some(Action::SettingsDown),
            _ => None,
        },
        SettingsMode::AddRoot => match key.code {
            Esc => Some(Action::ViewSettings),
            Enter => Some(Action::RootConfirm),
            _ => Some(Action::SettingsInput(*key)),
        },
        SettingsMode::RemoveRoot => match key.code {
            Esc => Some(Action::ViewSettings),
            Enter => Some(Action::RootConfirm),
            _ => None,
        },
    }
}

pub fn next_event() -> Result<Option<Event>> {
    match event::poll(Duration::from_millis(REFRESH_RATE))? {
        true => Ok(Some(event::read()?)),
        false => Ok(None),
    }
}

// .\src\key_handler\mod.rs
mod action;

pub use action::Action;
pub use action::Director;
pub use action::handle_key_event;
pub use action::next_event;

// .\src\lib.rs
use ratatui::crossterm::{
    cursor::MoveToColumn,
    style::Print,
    terminal::{Clear, ClearType},
    ExecutableCommand,
};
use std::{
    fs,
    io::Write,
    path::Path,
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

// .\src\library\library.rs
use super::LEGAL_EXTENSION;
use crate::{
    calculate_signature,
    database::Database,
    domain::{Album, LongSong, SimpleSong, SongInfo},
};
use anyhow::{anyhow, Context, Result};
use rayon::prelude::*;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use walkdir::WalkDir;

pub struct Library {
    db: Arc<Mutex<Database>>,
    pub roots: HashSet<PathBuf>,
    pub songs: Vec<Arc<SimpleSong>>,
    pub albums: Vec<Album>,
}

impl Library {
    fn new(db: Arc<Mutex<Database>>) -> Self {
        Library {
            db,
            roots: HashSet::new(),
            songs: Vec::new(),
            albums: Vec::new(),
        }
    }

    pub fn init(db: Arc<Mutex<Database>>) -> Self {
        let mut lib = Self::new(db);

        {
            let mut db = lib.db.lock().unwrap();

            if let Ok(db_roots) = db.get_roots() {
                for root in db_roots {
                    if let Ok(canon) = PathBuf::from(root).canonicalize() {
                        lib.roots.insert(canon);
                    }
                }
            }
        }

        lib
    }

    pub fn get_db(&self) -> Arc<Mutex<Database>> {
        Arc::clone(&self.db)
    }

    pub fn add_root(&mut self, root: impl AsRef<Path>) -> Result<()> {
        let canon = PathBuf::from(root.as_ref())
            .canonicalize()
            .map_err(|_| anyhow::format_err!("Path does not exist! {}", root.as_ref().display()))?;

        if self.roots.insert(canon.clone()) {
            let mut db = self.db.lock().unwrap();
            db.set_root(&canon)?;
        }

        Ok(())
    }

    pub fn delete_root(&mut self, root: &str) -> Result<()> {
        let bad_root = PathBuf::from(root);
        match self.roots.remove(&bad_root) {
            true => {
                let mut db = self.db.lock().unwrap();
                db.delete_root(&bad_root)?;
                Ok(())
            }
            false => Err(anyhow!("Error deleting root")),
        }
    }

    /// Build the library based on the current state of the database.
    pub fn build_library(&mut self) -> Result<()> {
        match self.roots.is_empty() {
            true => {}
            false => {
                self.update_db_by_root()?;
                self.collect_songs()?;
                self.build_albums()?;
            }
        }

        Ok(())
    }

    /// Walk through directories and update database based on changes made.
    pub fn update_db_by_root(&mut self) -> Result<(usize, usize)> {
        let mut db = self.db.lock().unwrap();
        let mut existing_hashes = db.get_hashes()?;
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
            Self::insert_new_songs(&mut db, new_files)?;
        }

        if !removed_ids.is_empty() {
            db.delete_songs(&removed_ids)?;
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
            .filter_map(|path| {
                // LongSong::build_song_ffprobe(&path)
                LongSong::build_song_symphonia(&path)
                    // .map_err(|e| println!("Error in file: {}\nERROR: {e}", path.display()))
                    .ok()
            })
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
        let mut db = self.db.lock().unwrap();
        self.songs = db.get_all_songs()?;

        Ok(())
    }

    fn build_albums(&mut self) -> Result<()> {
        let mut db = self.db.lock().unwrap();
        let aa_cache = db.get_album_map()?;

        self.albums = Vec::with_capacity(aa_cache.len());

        let mut album_lookup = HashMap::with_capacity(aa_cache.len());

        // Create album instances from album_artist/album_title combination
        for (album_name, artist_name) in &aa_cache {
            let album = Album::from_aa(album_name, artist_name);
            let idx = self.albums.len();
            self.albums.push(album);

            album_lookup.insert((Arc::clone(artist_name), Arc::clone(album_name)), idx);
        }

        // ASsign each song to it's proper album
        for song in &self.songs {
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

// Waveform related
impl Library {
    pub fn set_waveform(&mut self, id: u64, wf: &Vec<f32>) -> Result<()> {
        let mut db = self.db.lock().unwrap();
        Ok(db.set_waveform(id, wf)?)
    }

    pub fn get_waveform(&mut self, path: &str) -> Result<Vec<f32>> {
        let mut db = self.db.lock().unwrap();
        let waveform = db
            .get_waveform(path)
            .context("Failed to retrieve valid waveform data")?;
        Ok(waveform)
    }

    pub fn get_path(&self, id: u64) -> Result<String> {
        let mut this_db = Database::open()?;
        let str = Database::get_song_path(&mut this_db, id)?;

        Ok(str)
    }
}

impl Library {
    pub fn set_history_db(&self, history: &[Arc<SimpleSong>]) -> Result<()> {
        let mut db = self.db.lock().unwrap();

        db.save_history_to_db(history)
    }

    pub fn update_play_count(&self, song: &Arc<SimpleSong>) {
        let mut db = self.db.lock().unwrap();
        db.update_play_count(&song)
            .unwrap_or_else(|e| eprintln!("Error: {e}"));
    }

    pub fn load_history(&self, songs: &[Arc<SimpleSong>]) -> Result<VecDeque<Arc<SimpleSong>>> {
        let mut db = self.db.lock().unwrap();

        db.import_history(songs)
    }
}

// UI State
impl Library {
    pub fn get_all_songs(&self) -> &Vec<Arc<SimpleSong>> {
        &self.songs
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

// .\src\main.rs
fn main() -> anyhow::Result<()> {
    unsafe { std::env::set_var("RUST_BACKTRACE", "1") };
    concertus::app_core::Concertus::new().run()?;
    Ok(())
}

// .\src\player\command.rs
use crate::domain::QueueSong;
use std::sync::Arc;

pub enum PlayerCommand {
    Play(Arc<QueueSong>),
    TogglePlayback,
    SeekForward(usize),
    SeekBack(usize),
    Stop,
}

// .\src\player\controller.rs
use super::{PlaybackState, Player, PlayerCommand, PlayerState};
use crate::domain::{QueueSong, SimpleSong};
use anyhow::Result;
use std::{
    sync::{
        mpsc::{self, Sender},
        Arc, Mutex,
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
mod command;
mod controller;
mod player;
mod state;

pub use command::PlayerCommand;
pub use controller::PlayerController;
pub use player::Player;
pub use state::{PlaybackState, PlayerState};

// .\src\player\player.rs
use super::{PlaybackState, PlayerState};
use crate::domain::{FileType, QueueSong};
use anyhow::Result;
use rodio::{Decoder, OutputStream, Sink};
use std::{
    ops::Sub,
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
        let (_stream, stream_handle) =
            OutputStream::try_default().expect("Rodio: Could not create OutputStream.");
        let sink = Sink::try_new(&stream_handle).expect("Rodio: Could not create Sink.");
        Player {
            sink,
            _stream,
            shared_state,
        }
    }

    /// Play a song
    /// Return an error if
    pub(crate) fn play_song(&mut self, song: &Arc<QueueSong>) -> Result<()> {
        let file = std::fs::File::open(&song.path)?;
        let source = Decoder::new(std::io::BufReader::new(file))?;

        self.sink.stop();
        self.sink.append(source);
        self.sink.play();

        let mut player_state = self
            .shared_state
            .lock()
            .expect("Failed to unwrap mutex in music player");
        player_state.state = PlaybackState::Playing;
        player_state.now_playing = Some(Arc::clone(&song.meta));
        player_state.elapsed = Duration::default();

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
        self.sink.stop();

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

        if playback_state != PlaybackState::Stopped
            && (now_playing.as_deref().unwrap().format != FileType::OGG)
        {
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
                    self.sink.stop();
                    state.state = PlaybackState::Stopped;
                } else {
                    state.elapsed = self.sink.get_pos()
                }
            } else {
                self.sink.stop();
                state.state = PlaybackState::Stopped;
            }
        }
        Ok(())
    }

    /// Rewinds playback 5 seconds
    pub(crate) fn seek_back(&mut self, secs: usize) {
        let (now_playing, playback_state) = {
            let state = self
                .shared_state
                .lock()
                .expect("Failed to unwrap mutex in music player");
            (state.now_playing.clone(), state.state)
        };

        if playback_state != PlaybackState::Stopped
            && (now_playing.as_deref().unwrap().format != FileType::OGG)
        {
            let elapsed = self.sink.get_pos();

            if elapsed < Duration::from_secs(secs as u64) {
                let _ = self.sink.try_seek(Duration::from_secs(0));
            } else {
                let new_time = elapsed.sub(Duration::from_secs(secs as u64));
                let _ = self.sink.try_seek(new_time);
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
                state.elapsed = self.sink.get_pos()
            }
        }
    }

    pub(crate) fn sink_is_empty(&self) -> bool {
        self.sink.empty()
    }
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
}

impl Default for PlayerState {
    fn default() -> Self {
        PlayerState {
            now_playing: None,
            elapsed: Duration::default(),
            state: PlaybackState::Stopped,
            player_error: None,
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
}

impl AppLayout {
    pub fn new(area: Rect, state: &UiState) -> Self {
        let (wf_splitter, wf_height) = match state.get_now_playing().is_some() {
            true => (1, 6),
            false => (0, 0),
        };

        let (search_splitter, search_height) = match state.get_mode() == Mode::Search {
            true => (0, 5),
            false => (0, 0),
        };

        let [upper_block, _, progress_bar] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(12),
                Constraint::Length(wf_splitter),
                Constraint::Length(wf_height),
            ])
            .areas(area);

        let [sidebar, _, upper_block] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(40),
                Constraint::Length(1),
                Constraint::Min(40),
            ])
            .areas(upper_block);

        let [search_bar, _, song_window] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(search_height),
                Constraint::Length(search_splitter),
                Constraint::Fill(100),
            ])
            .areas(upper_block);

        AppLayout {
            sidebar,
            search_bar,
            song_window,
            progress_bar,
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
pub use widgets::SideBar;
pub use widgets::SongTable;
// pub use widgets::StandardTable;

use ratatui::widgets::Padding;
pub(crate) const SEARCH_PADDING: Padding = Padding {
    left: 2,
    right: 2,
    top: 1,
    bottom: 0,
};

// .\src\tui\renderer.rs
use super::widgets::Settings;
use super::{widgets::SongTable, AppLayout};
use super::{ErrorMsg, Progress, SearchBar, SideBar};
use crate::ui_state::Pane;
use crate::UiState;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    widgets::{Widget, *},
    Frame,
};

pub fn render(f: &mut Frame, state: &mut UiState) {
    let layout = AppLayout::new(f.area(), &state);

    SearchBar.render(layout.search_bar, f.buffer_mut(), state);
    SideBar.render(layout.sidebar, f.buffer_mut(), state);
    SongTable.render(layout.song_window, f.buffer_mut(), state);
    Progress.render(layout.progress_bar, f.buffer_mut(), state);

    // POPUPS AND ERRORS
    match (state.get_pane() == Pane::Popup, &state.get_error()) {
        (true, Some(_)) => {
            let error_win = centered_rect(40, 40, f.area());
            Clear.render(error_win, f.buffer_mut());
            ErrorMsg.render(error_win, f.buffer_mut(), state);
        }
        (true, None) => {
            let settings_popup = centered_rect(50, 50, f.area());
            Clear.render(settings_popup, f.buffer_mut());
            Settings.render(settings_popup, f.buffer_mut(), state);
        }
        (false, _) => (),
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

// fn get_up_next(state: &UiState) -> ratatui::text::Line<'_> {
//     ratatui::text::Line::from(match state.peek_queue() {
//         Some(s) => &s.title,
//         _ => "",
//     })
// }

// .\src\tui\widgets\error.rs
use crate::ui_state::UiState;
use ratatui::{
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
        let err_str = state
            .get_error()
            .as_ref()
            .unwrap_or(&anyhow::anyhow!("No error to display"))
            .to_string();

        Paragraph::new(err_str)
            .wrap(Wrap { trim: true })
            .centered()
            .block(
                Block::bordered()
                    .border_type(BorderType::Double)
                    .title_bottom(" Press <Esc> to clear ")
                    .title_alignment(ratatui::layout::Alignment::Center)
                    .padding(PADDING),
            )
            .bg(ratatui::style::Color::LightRed)
            .render(area, buf);
    }
}

// .\src\tui\widgets\mod.rs
mod error;
mod progress;
mod progress_bar;
mod search;
mod settings;
mod sidebar;
mod song_window;
mod tracklist;
mod waveform;

pub use error::ErrorMsg;
pub use progress::Progress;
pub use search::SearchBar;
pub use settings::Settings;
pub use sidebar::SideBar;
pub use song_window::SongTable;
pub use waveform::Waveform;

const DUR_WIDTH: u16 = 5;
const PAUSE_ICON: &str = "󰏤";
const WAVEFORM_WIDGET_HEIGHT: f64 = 50.0;

// .\src\tui\widgets\progress.rs
use crate::{tui::widgets::Waveform, ui_state::UiState};
use ratatui::widgets::StatefulWidget;

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

// .\src\tui\widgets\progress_bar.rs
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
                .padding(SEARCH_PADDING)
                .fg(theme.border),
        );

        search.render(area, buf);
    }
}

// .\src\tui\widgets\settings.rs
use crate::{
    strip_win_prefix,
    ui_state::{Pane, SettingsMode, UiState, GOOD_RED},
};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{
        Block, BorderType, HighlightSpacing, List, Padding, Paragraph, StatefulWidget, Widget, Wrap,
    },
};

static POPUP_PADDING: Padding = Padding {
    left: 2,
    right: 2,
    top: 1,
    bottom: 1,
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
            SettingsMode::ViewRoots => " Settings - Music Library Roots ",
            SettingsMode::AddRoot => " Add New Root Directory ",
            SettingsMode::RemoveRoot => " Remove Root Directory ",
        };

        let block = Block::bordered()
            .title(title)
            .title_bottom(get_help_text(&settings_mode))
            .title_alignment(ratatui::layout::Alignment::Center)
            .border_type(BorderType::Double)
            .border_style(Style::new().fg(Color::Rgb(255, 70, 70)))
            .bg(Color::Rgb(25, 25, 25))
            .padding(POPUP_PADDING);

        let inner = block.inner(area);
        block.render(area, buf);

        match settings_mode {
            SettingsMode::ViewRoots => render_roots_list(inner, buf, state),
            SettingsMode::AddRoot => render_add_root(inner, buf, state),
            SettingsMode::RemoveRoot => render_remove_root(inner, buf, state),
        }
    }
}

fn get_help_text(mode: &SettingsMode) -> &'static str {
    match mode {
        SettingsMode::ViewRoots => " [a]dd / [d]elete / [Esc] close ",
        SettingsMode::AddRoot => " [Enter] confirm / [Esc] cancel ",
        SettingsMode::RemoveRoot => " [Enter] confirm / [Esc] cancel ",
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

    let theme = state.get_theme(&Pane::Popup);

    let list = List::new(items)
        .highlight_style(Style::default().fg(Color::Black).bg(theme.text_highlighted))
        // .highlight_symbol(SELECTOR)
        .highlight_spacing(HighlightSpacing::Always);

    ratatui::prelude::StatefulWidget::render(
        list,
        area,
        buf,
        &mut state.settings.settings_selection,
    );
}

fn render_add_root(
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
    state: &mut UiState,
) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(1),
    ])
    .split(area);

    Paragraph::new("Enter the path to a directory containing music files:").render(chunks[0], buf);

    let theme = state.get_theme(&Pane::Popup);

    state.settings.root_input.set_block(
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
        .settings
        .root_input
        .set_style(Style::new().fg(theme.text_focused));

    state.settings.root_input.render(chunks[1], buf);

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
    let selected_root = &roots[state.settings.settings_selection.selected().unwrap()];
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

// .\src\tui\widgets\sidebar.rs
use crate::ui_state::{AlbumDisplayItem, AlbumSort, Pane, UiState, GOLD_FADED};
use ratatui::{
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, List, ListItem, ListState, Padding, StatefulWidget},
};

pub struct SideBar;
impl StatefulWidget for SideBar {
    type State = UiState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let albums = &state.albums;
        let pane_title = format!(" ⟪ {} Albums! ⟫ ", albums.len());
        let pane_org = state.get_album_sort_string();
        let pane_org = format!("{pane_org:5} ");

        let theme = &state.get_theme(&Pane::SideBar);

        // Get the currently selected artist (if any)
        let selected_artist = state
            .get_selected_album()
            .map(|album| album.artist.as_str());

        // Create list items from display items
        let display_items = &state.display_state.album_headers;

        let list_items = display_items
            .iter()
            .map(|item| match item {
                AlbumDisplayItem::Header(artist) => {
                    let is_selected_artist = selected_artist.map_or(false, |sel| sel == artist);

                    let style = match is_selected_artist {
                        true => Style::default()
                            .fg(theme.text_highlighted)
                            .add_modifier(Modifier::ITALIC | Modifier::UNDERLINED),
                        false => Style::default().fg(GOLD_FADED),
                    };

                    ListItem::new(Span::from(format!("{}", artist)).italic().style(style))
                }
                AlbumDisplayItem::Album(idx) => {
                    let album = &albums[*idx];

                    let year = match album.year {
                        Some(y) => format!("{y}"),
                        _ => String::from("----"),
                    };

                    let indent = match state.get_album_sort() == AlbumSort::Artist {
                        true => "  ",
                        false => "",
                    };

                    let year_txt =
                        Span::from(format!("{}{: >4} ", indent, year)).fg(theme.text_secondary);
                    let separator = Span::from("✧ ").fg(theme.text_faded);
                    let album_title = Span::from(album.title.as_str()).fg(theme.text_focused);

                    ListItem::new(Line::from_iter([year_txt, separator, album_title]))
                }
            })
            .collect::<Vec<ListItem>>();

        let display_selected = if let Some(album_idx) = state.display_state.sidebar_pos.selected() {
            state
                .display_state
                .album_headers
                .iter()
                .position(|item| match item {
                    AlbumDisplayItem::Album(idx) => *idx == album_idx,
                    _ => false,
                })
        } else {
            None
        };

        // Create a temporary display state
        let mut display_state = ListState::default();
        display_state.select(display_selected);
        *display_state.offset_mut() = state.display_state.sidebar_pos.offset();

        let current_offset = state.display_state.sidebar_pos.offset();
        *display_state.offset_mut() = current_offset;

        // Ensure header is visible
        if state.get_album_sort() == AlbumSort::Artist && display_selected.is_some() {
            let display_idx = display_selected.unwrap();

            // Get album header
            let mut header_idx = display_idx;
            while header_idx > 0 {
                header_idx -= 1;
                if let AlbumDisplayItem::Header(_) = display_items[header_idx] {
                    break;
                }
            }

            if header_idx < current_offset && display_idx >= current_offset {
                *display_state.offset_mut() = header_idx;
            }
        }

        let keymaps = match state.get_pane() {
            Pane::SideBar => Line::from(" [q] Queue Album ")
                .centered()
                .fg(theme.text_faded),
            _ => Line::default(),
        };

        let block = Block::bordered()
            // .borders(theme.border_display)
            .border_type(BorderType::Thick)
            .border_style(theme.border)
            .bg(theme.bg)
            .title_top(Line::from(pane_title).left_aligned().fg(theme.text_focused))
            .title_top(
                Line::from_iter([" 󰒿 ", &pane_org])
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
            .scroll_padding(4);

        list.render(area, buf, &mut display_state);
        *state.display_state.sidebar_pos.offset_mut() = display_state.offset();
    }
}

// .\src\tui\widgets\song_window.rs
use ratatui::widgets::StatefulWidget;

use crate::ui_state::{Mode, UiState};

use super::tracklist::{AlbumView, QueueTable, StandardTable};

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
            &Mode::Album => AlbumView.render(area, buf, state),
            &Mode::Queue => QueueTable.render(area, buf, state),
            _ => StandardTable.render(area, buf, state),
        }
    }
}

// .\src\tui\widgets\tracklist\album_view.rs
use crate::{
    domain::{SimpleSong, SongInfo},
    get_readable_duration, truncate_at_last_space,
    ui_state::{DisplayTheme, Pane, UiState},
    DurationStyle,
};
use ratatui::{
    layout::{Alignment, Flex},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{StatefulWidget, *},
};
use std::sync::Arc;
use std::{
    collections::{HashMap, HashSet},
    sync::LazyLock,
};

use super::{get_header, get_widths, COLUMN_SPACING, PADDING, SELECTOR};

static SUPERSCRIPT: std::sync::LazyLock<std::collections::HashMap<u32, &str>> =
    LazyLock::new(|| {
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

pub struct AlbumView;
impl StatefulWidget for AlbumView {
    type State = UiState;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        if state.albums.is_empty() {
            return;
        }

        let theme = &state.get_theme(&Pane::TrackList);
        let album = state
            .get_selected_album()
            .unwrap_or(&state.albums[0])
            .clone();

        let album_title = truncate_at_last_space(&album.title, (area.width / 3) as usize);

        let queued_ids: HashSet<u64> = state.playback.queue.iter().map(|s| s.get_id()).collect();
        let now_playing_id = state.get_now_playing().map(|s| s.id);

        let disc_count = album
            .tracklist
            .iter()
            .filter_map(|s| s.disc_no)
            .max()
            .unwrap_or(1) as usize;

        let rows = album
            .tracklist
            .iter()
            .map(|song| {
                let is_queued = queued_ids.contains(&song.id);
                let is_playing = now_playing_id == Some(song.id);

                let title_cell = match (is_queued, is_playing) {
                    (true, false) => Line::from_iter([
                        song.get_title().fg(theme.text_focused),
                        " [queued]".fg(theme.text_faded).italic().into(),
                    ]),
                    (false, true) => Line::from_iter([
                        song.get_title().fg(theme.text_focused),
                        " ♫".fg(theme.text_secondary).into(),
                    ]),
                    _ => Line::from_iter([song.get_title().fg(theme.text_focused)]),
                };

                let track_no_cell = get_track_discs(song, disc_count, theme);
                let artist_cell = Cell::from(song.get_artist()).fg(theme.text_focused);
                let format = Cell::from(format!("{}", song.format)).fg(theme.text_secondary);
                let duration_str = get_readable_duration(song.duration, DurationStyle::Clean);
                let duration_cell =
                    Cell::from(Text::from(duration_str).right_aligned()).fg(theme.text_focused);

                Row::new([
                    track_no_cell,
                    title_cell.into(),
                    artist_cell,
                    format,
                    duration_cell,
                ])
            })
            .collect::<Vec<Row>>();

        let year_str = album
            .year
            .filter(|y| *y != 0)
            .map_or(String::new(), |y| format!("[{y}]"));

        let title_line = Line::from_iter([
            Span::from(format!(" {} ", album_title))
                .fg(theme.text_secondary)
                .italic(),
            Span::from(year_str).fg(theme.text_faded),
            Span::from(" ✧ ").fg(theme.text_faded),
            Span::from(album.artist.as_str()).fg(theme.text_focused),
            Span::from(format!(" [{} Songs] ", album.tracklist.len())).fg(theme.text_faded),
        ]);

        let header = get_header(&state.get_mode(), &state.get_table_sort());
        let widths = get_widths(&state.get_mode());

        let keymaps = match state.get_pane() {
            Pane::TrackList => " [q] Queue Song ✧ [Tab] Back ".fg(theme.text_faded),
            _ => "".into(),
        };

        let block = Block::bordered()
            .title_top(title_line)
            .title_bottom(keymaps)
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Thick)
            .border_style(Style::default().fg(theme.border))
            .bg(theme.bg)
            .padding(PADDING);

        let table = Table::new(rows, widths)
            .header(
                Row::new(header)
                    .fg(theme.text_secondary)
                    .bottom_margin(1)
                    .bold(),
            )
            .column_spacing(COLUMN_SPACING)
            .flex(Flex::Start)
            .block(block)
            .highlight_symbol(SELECTOR)
            .highlight_spacing(HighlightSpacing::Always)
            .row_highlight_style(
                Style::default()
                    .bg(theme.text_highlighted)
                    .fg(Color::Black)
                    .italic(),
            );

        // RENDER THE TABLE
        StatefulWidget::render(table, area, buf, &mut state.display_state.table_pos);
    }
}

fn get_track_discs(
    song: &Arc<SimpleSong>,
    disc_count: usize,
    theme: &DisplayTheme,
) -> Cell<'static> {
    let track_no = Span::from(match song.track_no {
        Some(t) => format!("{t:>2}"),
        None => format!("{x:>2}", x = "󰇘"),
    })
    .fg(theme.text_highlighted);

    let disc_no = Span::from(match disc_count {
        0..2 => "".to_string(),
        _ => match song.disc_no {
            Some(t) => String::from("ᴰ") + SUPERSCRIPT.get(&t).unwrap_or(&"?"),
            None => "".into(),
        },
    })
    .fg(theme.text_faded);

    Cell::from(Line::from_iter([track_no, " ".into(), disc_no.into()]))
}

// .\src\tui\widgets\tracklist\mod.rs
mod album_view;
mod queue_view;
mod search_results;

pub use album_view::AlbumView;
pub use queue_view::QueueTable;
pub use search_results::StandardTable;

use crate::ui_state::{Mode, TableSort};
use ratatui::{
    layout::Constraint,
    style::{Color, Stylize},
    text::{Span, Text},
    widgets::Padding,
};

const COLUMN_SPACING: u16 = 2;
const SELECTOR: &str = "⮞  ";

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
                Constraint::Ratio(3, 9),
                Constraint::Ratio(2, 9),
                Constraint::Ratio(2, 9),
                Constraint::Length(8),
            ]
        }
        Mode::Album => {
            vec![
                Constraint::Length(6),
                Constraint::Min(25),
                Constraint::Max(30),
                Constraint::Max(6),
                Constraint::Length(7),
            ]
        }
        Mode::Queue => {
            vec![
                Constraint::Min(3),
                Constraint::Min(30),
                Constraint::Fill(30),
                Constraint::Max(5),
                Constraint::Max(6),
            ]
        }
        _ => Vec::new(),
    }
}

pub(super) fn get_header<'a>(mode: &Mode, active: &TableSort) -> Vec<Text<'a>> {
    match mode {
        Mode::Power | Mode::Search => [
            TableSort::Title,
            TableSort::Artist,
            TableSort::Album,
            TableSort::Duration,
        ]
        .iter()
        .map(|s| match (s == active, s.eq(&TableSort::Duration)) {
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
        })
        .collect(),
        Mode::Album => {
            vec![
                Text::default(),
                Text::from("Title").underlined(),
                Text::from("Artist").underlined(),
                Text::from("Format").underlined(),
                Text::from("Length").right_aligned().underlined(),
            ]
        }
        _ => Vec::new(),
    }
}

// .\src\tui\widgets\tracklist\queue_view.rs
use super::{get_widths, COLUMN_SPACING, PADDING, SELECTOR};
use crate::{
    domain::SongInfo,
    get_readable_duration,
    ui_state::{Pane, UiState},
    DurationStyle,
};
use ratatui::{
    layout::{Alignment, Flex},
    style::{Style, Stylize},
    text::{Line, Text},
    widgets::{StatefulWidget, *},
};

pub struct QueueTable;
impl StatefulWidget for QueueTable {
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

        let results = format!(" Queue Size: {} Songs ", song_len);

        let rows = songs
            .iter()
            .enumerate()
            .map(|(idx, song)| {
                let index = Cell::from(format!("{:>3}", idx + 1)).fg(theme.text_faded);
                let title_col = Cell::from(song.get_title()).fg(theme.text_focused);
                let artist_col = Cell::from(song.get_artist()).fg(theme.text_focused);
                let format_col = Cell::from(song.format.to_string()).fg(theme.text_secondary);
                let duration_str = get_readable_duration(song.duration, DurationStyle::Clean);

                let dur_col =
                    Cell::from(Text::from(duration_str).right_aligned()).fg(theme.text_focused);

                Row::new([index, title_col, artist_col, format_col, dur_col])
            })
            .collect::<Vec<Row>>();

        let widths = get_widths(&state.get_mode());

        let block = Block::bordered()
            .title_top(Line::from(results).alignment(Alignment::Center))
            .borders(theme.border_display)
            .border_type(BorderType::Thick)
            .border_style(Style::default().fg(theme.border))
            .bg(theme.bg)
            .padding(PADDING);

        let table = Table::new(rows, widths)
            .column_spacing(COLUMN_SPACING)
            .flex(Flex::SpaceAround)
            .block(block)
            .highlight_spacing(HighlightSpacing::Always)
            .row_highlight_style(theme.text_highlighted)
            .highlight_symbol(SELECTOR);

        StatefulWidget::render(table, area, buf, &mut state.display_state.table_pos);
    }
}

// .\src\tui\widgets\tracklist\search_results.rs
use super::{get_header, get_widths, COLUMN_SPACING, PADDING, SELECTOR};
use crate::{
    domain::SongInfo,
    ui_state::{Pane, TableSort, UiState},
};
use ratatui::{
    layout::{Alignment, Flex},
    style::{Color, Style, Stylize},
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

        let results = match state.get_mode() {
            _ => match search_len > 1 {
                true => format!(" Search Results: {} Songs ", song_len),
                false => format!(" Total: {} Songs ", song_len),
            },
        };

        let rows = songs
            .iter()
            .map(|song| {
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
                Row::new([title_col, artist_col, album_col, dur_col])
            })
            .collect::<Vec<Row>>();

        let header = Row::new(get_header(&state.get_mode(), &state.get_table_sort()))
            .bold()
            .fg(theme.text_secondary)
            .bottom_margin(1);
        let widths = get_widths(&state.get_mode());

        let block = Block::bordered()
            .title_top(Line::from(results).alignment(Alignment::Center))
            .border_style(theme.border)
            .border_type(BorderType::Thick)
            .padding(PADDING)
            .fg(theme.text_focused)
            .bg(theme.bg);

        let table = Table::new(rows, widths)
            .column_spacing(COLUMN_SPACING)
            .header(header)
            .flex(Flex::Legacy)
            .block(block)
            .row_highlight_style(
                Style::default()
                    .bg(theme.text_highlighted)
                    .fg(Color::Black)
                    .italic(),
            )
            .highlight_spacing(HighlightSpacing::Always)
            .highlight_symbol(SELECTOR);

        StatefulWidget::render(table, area, buf, &mut state.display_state.table_pos);
    }
}

// .\src\tui\widgets\waveform.rs
use super::{DUR_WIDTH, WAVEFORM_WIDGET_HEIGHT};
use crate::{domain::SongInfo, get_readable_duration, ui_state::UiState, DurationStyle};
use canvas::Context;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Stylize},
    text::{Line, Span, Text},
    widgets::{
        canvas::{Canvas, Rectangle},
        StatefulWidget, *,
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
        let theme = &state.get_theme(state.get_pane());

        let playing_title = Line::from_iter([
            Span::from(np.get_title()).fg(theme.text_secondary),
            Span::from(" ✧ ").fg(theme.text_faded),
            Span::from(np.get_artist()).fg(theme.text_faded),
        ]);

        let waveform = state.get_waveform();
        let wf_len = waveform.len();

        let x_duration = area.width - 8;
        let y = buf.area().height
            - match area.height {
                0 => 1,
                _ => area.height / 2 + 1,
            };

        let elapsed_str =
            get_readable_duration(state.get_playback_elapsed(), DurationStyle::Compact);

        let duration_str = get_readable_duration(np.get_duration(), DurationStyle::Compact);

        Text::from(elapsed_str)
            .fg(Color::DarkGray)
            .right_aligned()
            .render(Rect::new(2, y, DUR_WIDTH, 1), buf);

        Text::from(duration_str)
            .fg(Color::DarkGray)
            .right_aligned()
            .render(Rect::new(x_duration, y, DUR_WIDTH, 1), buf);

        Canvas::default()
            .x_bounds([0.0, wf_len as f64])
            .y_bounds([WAVEFORM_WIDGET_HEIGHT * -1.0, WAVEFORM_WIDGET_HEIGHT])
            // .background_color(Color::DarkGray)
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
            .block(
                Block::new()
                    .title_bottom(playing_title.alignment(Alignment::Center))
                    .padding(Padding {
                        left: 10,
                        right: 10,
                        top: 0,
                        bottom: 0,
                    }),
            )
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
use super::{AlbumDisplayItem, AlbumSort, Mode, Pane, TableSort, UiState};
use crate::{
    domain::{Album, SimpleSong, SongInfo},
    key_handler::Director,
};
use anyhow::{anyhow, Result};
use ratatui::widgets::{ListState, TableState};
use std::{ops::Index, sync::Arc};

pub struct DisplayState {
    mode: Mode,
    pane: Pane,

    table_sort: TableSort,
    pub(super) album_sort: AlbumSort,

    pub sidebar_pos: ListState,
    pub table_pos: TableState,

    sidebar_pos_cached: usize,
    table_pos_cached: usize,

    pub album_headers: Vec<AlbumDisplayItem>,
}

impl DisplayState {
    pub fn new() -> Self {
        DisplayState {
            mode: Mode::Album,
            pane: Pane::TrackList,
            table_sort: TableSort::Title,
            album_sort: AlbumSort::Artist,
            table_pos: TableState::default().with_selected(0),
            table_pos_cached: 0,
            sidebar_pos: ListState::default().with_selected(Some(0)),
            sidebar_pos_cached: 0,
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

    pub fn set_mode(&mut self, mode: Mode) {
        match self.display_state.mode {
            Mode::Power => {
                self.display_state.table_pos_cached = self
                    .display_state
                    .table_pos
                    .selected()
                    .unwrap_or(self.display_state.table_pos_cached)
            }
            Mode::Album => {
                self.display_state.sidebar_pos_cached = self
                    .display_state
                    .sidebar_pos
                    .selected()
                    .unwrap_or(self.display_state.sidebar_pos_cached)
            }
            _ => (),
        }

        match mode {
            Mode::Power => {
                self.display_state.mode = Mode::Power;
                self.display_state.pane = Pane::TrackList;
                self.display_state.table_sort = TableSort::Title;
                self.display_state
                    .table_pos
                    .select(Some(self.display_state.table_pos_cached));
            }
            Mode::Album => {
                self.display_state.mode = Mode::Album;
                self.display_state.pane = Pane::SideBar;
                match self.albums.is_empty() {
                    true => self.display_state.sidebar_pos.select(None),
                    false => self
                        .display_state
                        .sidebar_pos
                        .select(Some(self.display_state.sidebar_pos_cached)),
                }
                *self.display_state.table_pos.offset_mut() = 0;
                self.set_legal_songs();
            }
            Mode::Playlist => {}
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
            Mode::Power | Mode::Album | Mode::Search => {
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

        self.update_album_sidebar_view();
    }

    fn update_album_sidebar_view(&mut self) {
        self.display_state.album_headers.clear();

        if self.display_state.album_sort == AlbumSort::Artist {
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
        } else {
            // For other sort types, just add albums without headers
            for (idx, _) in self.albums.iter().enumerate() {
                self.display_state
                    .album_headers
                    .push(AlbumDisplayItem::Album(idx));
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

        self.set_mode(Mode::Album);
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
            Mode::Album => {
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

        // Autoselect first entry if necessary
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
            if self.display_state.mode == Mode::Album {
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

// .\src\ui_state\mod.rs
mod album_sort;
mod display_state;
mod mode;
mod pane;
mod playback;
mod search_state;
mod settings;
mod table_sort;
mod theme;
mod ui_snapshot;
mod ui_state;

pub use album_sort::AlbumSort;
pub use display_state::DisplayState;
pub use mode::Mode;
pub use pane::Pane;
pub use settings::SettingsMode;
pub use table_sort::TableSort;
pub use theme::DisplayTheme;
pub use ui_snapshot::UiSnapshot;
pub use ui_state::UiState;

pub use theme::*;

pub enum AlbumDisplayItem {
    Header(String),
    Album(usize),
}

fn new_textarea(placeholder: &str) -> tui_textarea::TextArea<'static> {
    let mut search = tui_textarea::TextArea::default();
    search.set_cursor_line_style(ratatui::style::Style::default());
    search.set_placeholder_text(format!(" {placeholder}: "));

    search
}

// .\src\ui_state\mode.rs
#[derive(Default, PartialEq, Eq)]
pub enum Mode {
    Power,
    Playlist,
    Queue,
    Search,
    QUIT,
    #[default]
    Album,
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
            Mode::Album => write!(f, "album"),
            Mode::Playlist => write!(f, "playlist"),
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
            "album" => Mode::Album,
            "playlist" => Mode::Playlist,
            "queue" => Mode::Queue,
            "search" => Mode::Search,
            "quit" => Mode::QUIT,
            _ => Mode::Album,
        }
    }
}

// .\src\ui_state\pane.rs
#[derive(Default, PartialEq, Eq)]
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
            Pane::Popup => write!(f, "popup"),
            Pane::Search => write!(f, "search"),
        }
    }
}

impl Pane {
    pub fn from_str(s: &str) -> Self {
        match s {
            "tracklist" => Pane::TrackList,
            "sidebar" => Pane::SideBar,
            "popup" => Pane::Popup,
            "search" => Pane::Search,
            _ => Pane::TrackList,
        }
    }
}

// .\src\ui_state\playback.rs
use super::{Mode, UiState};
use crate::{
    domain::{QueueSong, SimpleSong},
    player::{PlaybackState, PlayerState},
    strip_win_prefix,
};
use anyhow::{anyhow, Context, Result};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};

const HISTORY_CAPACITY: usize = 50;
pub struct PlaybackCoordinator {
    pub queue: VecDeque<Arc<QueueSong>>,
    pub history: VecDeque<Arc<SimpleSong>>,
    pub waveform: Vec<f32>,
    pub(self) player_state: Arc<Mutex<PlayerState>>,
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

    pub(crate) fn queue_song(&mut self, song: Option<Arc<SimpleSong>>) -> Result<()> {
        let simple_song = match song {
            Some(s) => s,
            None => self.get_selected_song()?,
        };

        let queue_song = self.make_playable_song(&simple_song)?;
        self.playback.queue.push_back(queue_song);
        Ok(())
    }

    pub fn queue_album(&mut self) -> Result<()> {
        let album = self
            .display_state
            .sidebar_pos
            .selected()
            .ok_or_else(|| anyhow::anyhow!("Illegal album selection!"))?;

        let songs = self.albums[album].tracklist.clone();
        for song in songs {
            self.queue_song(Some(song))?;
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
        self.playback.history = self
            .library
            .load_history(&self.library.get_all_songs())
            .unwrap_or_default();
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

    pub fn remove_from_queue(&mut self) -> Result<()> {
        if Mode::Queue == *self.get_mode() {
            self.display_state
                .table_pos
                .selected()
                .and_then(|idx| self.playback.queue.remove(idx))
                .map(|_| {
                    self.set_legal_songs();
                });
        }
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

// ============
//   WAVEFORM
// ==========
impl UiState {
    pub fn get_waveform(&self) -> &[f32] {
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

// .\src\ui_state\search_state.rs
use super::{new_textarea, Pane, UiState};
use crate::domain::{SimpleSong, SongInfo};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use ratatui::crossterm::event::KeyEvent;
use std::sync::Arc;
use tui_textarea::TextArea;

const MATCH_THRESHOLD: i64 = 50;

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
    pub(crate) fn filter_songs_by_search(&mut self) {
        let query = self.read_search().to_lowercase();

        let mut scored_songs: Vec<(Arc<SimpleSong>, i64)> = self
            .library
            .get_all_songs()
            .iter()
            .filter_map(|song| {
                self.search
                    .matcher
                    .fuzzy_match(&song.get_title().to_lowercase(), &query.as_str())
                    .filter(|&score| score > MATCH_THRESHOLD)
                    .map(|score| (song.clone(), score))
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
        if self.legal_songs.is_empty() {
            self.display_state.table_pos.select(None);
        } else {
            self.display_state.table_pos.select(Some(0));
        }
    }

    pub fn read_search(&self) -> &str {
        &self.search.input.lines()[0]
    }
}

// .\src\ui_state\settings\mod.rs
mod root_mgmt;

pub use root_mgmt::Settings;
pub use root_mgmt::SettingsMode;

// .\src\ui_state\settings\root_mgmt.rs
use crate::{
    app_core::Concertus,
    ui_state::{new_textarea, Pane, UiState},
    Library,
};
use anyhow::{anyhow, Result};
use ratatui::widgets::ListState;
use std::sync::Arc;
use tui_textarea::TextArea;

#[derive(Default, PartialEq, Clone)]
pub enum SettingsMode {
    #[default]
    ViewRoots,
    AddRoot,
    RemoveRoot,
}

pub struct Settings {
    pub settings_mode: SettingsMode,
    pub settings_selection: ListState,
    pub root_input: TextArea<'static>,
}

impl Settings {
    pub fn new() -> Self {
        Settings {
            settings_mode: SettingsMode::default(),
            settings_selection: ListState::default().with_selected(Some(0)),
            root_input: new_textarea("Enter path to directory"),
        }
    }
}

impl UiState {
    pub fn get_settings_mode(&self) -> &SettingsMode {
        &self.settings.settings_mode
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
        let db = self.library.get_db();
        let mut lib = Library::init(db);
        lib.add_root(path)?;
        lib.build_library()?;

        self.library = Arc::new(lib);

        Ok(())
    }

    pub fn remove_root(&mut self) -> Result<()> {
        if let Some(selected) = self.settings.settings_selection.selected() {
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
        self.settings.settings_mode = SettingsMode::ViewRoots;
        if !self.get_roots().is_empty() {
            self.settings.settings_selection.select(Some(0));
        }
        self.settings.root_input.select_all();
        self.settings.root_input.cut();
        self.set_pane(Pane::Popup);
    }
}

impl Concertus {
    pub(crate) fn settings_remove_root(&mut self) {
        if !self.ui.get_roots().is_empty() {
            self.ui.settings.settings_mode = SettingsMode::RemoveRoot;
        }
    }

    pub(crate) fn activate_settings(&mut self) {
        match self.ui.get_pane() {
            Pane::Popup => self.ui.soft_reset(),
            _ => {
                self.ui.set_pane(Pane::Popup);
                self.ui.settings.settings_mode = SettingsMode::ViewRoots
            }
        }
    }

    pub(crate) fn settings_scroll_up(&mut self) {
        let roots_count = self.ui.get_roots().len();
        if roots_count > 0 {
            let current = self.ui.settings.settings_selection.selected().unwrap_or(0);
            let new_selection = if current > 0 {
                current - 1
            } else {
                roots_count - 1 // Wrap to bottom
            };
            self.ui
                .settings
                .settings_selection
                .select(Some(new_selection));
        }
    }

    pub(crate) fn settings_scroll_down(&mut self) {
        let roots_count = self.ui.get_roots().len();
        if roots_count > 0 {
            let current = self.ui.settings.settings_selection.selected().unwrap_or(0);
            let new_selection = (current + 1) % roots_count; // Wrap to top
            self.ui
                .settings
                .settings_selection
                .select(Some(new_selection));
        }
    }

    pub(crate) fn settings_add_root(&mut self) {
        self.ui.settings.settings_mode = SettingsMode::AddRoot;
        self.ui.settings.root_input.select_all();
        self.ui.settings.root_input.cut();
    }

    pub(crate) fn settings_root_confirm(&mut self) -> anyhow::Result<()> {
        match self.ui.settings.settings_mode {
            SettingsMode::AddRoot => {
                let path = self.ui.settings.root_input.lines();
                let path = path[0].clone();
                if !path.is_empty() {
                    if let Err(e) = self.ui.add_root(&path) {
                        self.ui.set_error(e);
                    } else {
                        self.ui.settings.settings_mode = SettingsMode::ViewRoots;
                        self.update_library()?;
                    }
                }
            }
            SettingsMode::RemoveRoot => {
                if let Err(e) = self.ui.remove_root() {
                    self.ui.set_error(e);
                } else {
                    self.ui.settings.settings_mode = SettingsMode::ViewRoots;
                    self.ui.settings.settings_selection.select(Some(0));
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

// .\src\ui_state\ui_snapshot.rs
use crate::Database;
use anyhow::Result;

use super::{AlbumSort, Mode, Pane, UiState};

#[derive(Default)]
pub struct UiSnapshot {
    pub mode: String,
    pub pane: String,
    pub album_sort: String,
    pub album_selection: Option<usize>,
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
                "ui_song_pos" => snapshot.song_selection = value.parse().ok(),
                _ => {}
            }
        }

        snapshot
    }
}

impl UiState {
    pub fn create_snapshot(&self) -> UiSnapshot {
        UiSnapshot {
            mode: self.get_mode().to_string(),
            pane: self.get_pane().to_string(),
            album_sort: self.display_state.album_sort.to_string(),
            album_selection: self.display_state.sidebar_pos.selected(),
            song_selection: self.display_state.table_pos.selected(),
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
            self.display_state.album_sort = AlbumSort::from_str(&snapshot.album_sort);

            self.sort_albums();

            if let Some(pos) = snapshot.album_selection {
                if pos < self.albums.len() {
                    self.display_state.sidebar_pos.select(Some(pos));
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
use super::{
    playback::PlaybackCoordinator, search_state::SearchState, settings::Settings, theme::Theme,
    DisplayState, DisplayTheme, Mode, Pane,
};
use crate::{
    domain::{Album, SimpleSong},
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
                self.set_mode(Mode::Album);
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

