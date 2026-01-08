use crate::{
    CONFIG_DIRECTORY, DATABASE_FILENAME, SongMap,
    database::tables::CREATE_TABLES,
    library::{LongSong, SimpleSong, SongInfo},
};
use anyhow::Result;
use queries::*;
use rusqlite::{Connection, params};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs::{self},
    path::PathBuf,
    sync::Arc,
    time::{Duration, UNIX_EPOCH},
};

mod playlists;
mod queries;
mod snapshot;
mod tables;
mod worker;

pub(crate) const DB_BOUND: usize = 100;

pub use worker::DbWorker;

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
        tx.execute_batch(&CREATE_TABLES)?;
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
                    &song.channels,
                    &song.bit_rate,
                    &song.sample_rate,
                    &song.filetype
                ])?;
            }
        }
        tx.commit()?;

        Ok(())
    }

    pub(crate) fn get_all_songs(&mut self) -> Result<SongMap> {
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
                    album_id,
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
        self.conn.execute(UPDATE_PLAY_COUNT, params![id])?;

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

    pub(crate) fn get_album_map(&mut self) -> Result<Vec<(i64, Arc<String>, Arc<String>)>> {
        let map = self
            .conn
            .prepare(ALBUM_BUILDER)?
            .query_map([], |row| {
                let album_id = row.get("id")?;
                let artist_id = row.get("artist_id")?;

                let artist = match self.artist_map.get(&artist_id) {
                    Some(a) => Arc::clone(&a),
                    None => unreachable!(),
                };
                let album = match self.album_map.get(&album_id) {
                    Some(a) => Arc::clone(&a),
                    None => unreachable!(),
                };

                Ok((album_id, album, artist))
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

        let waveform = blob
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap()))
            .collect();

        Ok(waveform)
    }

    pub fn set_waveform(&mut self, id: u64, wf: &[f32]) -> Result<()> {
        let bytes: Vec<u8> = wf.iter().flat_map(|&f| f.to_le_bytes()).collect();

        self.conn
            .execute(INSERT_WAVEFORM, params![id.to_le_bytes(), bytes])?;

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

    pub fn import_history(&mut self, song_map: &SongMap) -> Result<VecDeque<Arc<SimpleSong>>> {
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
