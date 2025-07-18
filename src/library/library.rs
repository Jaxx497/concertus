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
