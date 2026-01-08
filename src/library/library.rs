use super::LEGAL_EXTENSION;
use crate::{
    SongMap,
    app_core::LibraryRefreshProgress,
    calculate_signature,
    database::Database,
    expand_tilde,
    library::{Album, LongSong, SimpleSong, SongInfo},
};

use anyhow::{Result, anyhow};
use crossbeam_channel::Sender;
use indexmap::IndexMap;
use rayon::prelude::*;
use std::{
    collections::{HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use walkdir::WalkDir;

pub struct Library {
    db: Database,
    pub roots: HashSet<PathBuf>,
    pub songs: SongMap,
    pub albums: IndexMap<i64, Album>,
}

const SCANNING_FINISHED: u8 = 25;
const PROCESSING_FINISHED: u8 = 70;
const REMOVALS_FINISHED: u8 = 90;

impl Library {
    fn new() -> Self {
        let db = Database::open().expect("Failed to connect to database!");
        Library {
            db,
            roots: HashSet::new(),
            songs: SongMap::default(),
            albums: IndexMap::new(),
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
        self.roots.remove(&bad_root);
        self.db.delete_root(&bad_root)
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
            .filter_map(|path| LongSong::build_song_lofty(&path).ok())
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

    pub fn get_songs_map(&self) -> &SongMap {
        &self.songs
    }

    pub fn get_song_by_id(&self, id: u64) -> Option<&Arc<SimpleSong>> {
        self.songs.get(&id)
    }

    fn build_albums(&mut self) -> Result<()> {
        let aa_cache = self.db.get_album_map()?;
        self.albums = IndexMap::with_capacity(aa_cache.len());

        // Create album instances from album_artist/album_title combination
        for (album_id, album_name, artist_name) in aa_cache {
            let album = Album::from_aa(album_id, album_name, artist_name);
            self.albums.insert(album_id, album);
        }

        let mut album_songs: IndexMap<i64, Vec<Arc<SimpleSong>>> =
            IndexMap::with_capacity(self.albums.len());

        for song in self.songs.values() {
            album_songs
                .entry(song.album_id)
                .or_insert_with(Vec::new)
                .push(Arc::clone(&song));
        }

        for (album_id, mut songs) in album_songs {
            if let Some(album) = self.albums.get_mut(&album_id) {
                if !songs.is_empty() {
                    if album.year.is_none() {
                        album.year = songs[0].year
                    }

                    songs.sort_by_key(|s| (s.disc_no.unwrap_or(0), s.track_no.unwrap_or(0)));
                    album.tracklist = songs.into()
                }
            }
        }

        self.albums.retain(|_id, album| !album.tracklist.is_empty());

        Ok(())
    }
}

impl Library {
    pub fn set_history_db(&mut self, history: &[u64]) -> Result<()> {
        self.db.save_history_to_db(history)
    }

    pub fn load_history(&mut self, songs: &SongMap) -> Result<VecDeque<Arc<SimpleSong>>> {
        self.db.import_history(songs)
    }

    pub fn get_all_songs(&self) -> Vec<Arc<SimpleSong>> {
        self.songs.values().cloned().collect()
    }
}

impl Library {
    pub fn build_library_with_progress(
        &mut self,
        tx: &Sender<LibraryRefreshProgress>,
    ) -> Result<()> {
        if self.roots.is_empty() {
            return Ok(());
        }

        // Phase 1: Scanning directories
        let _ = tx.send(LibraryRefreshProgress::Scanning { progress: 0 });

        let mut existing_hashes = self.db.get_hashes()?;
        let mut all_files = Vec::new();

        // First pass: collect all files from all roots
        for root in &self.roots {
            let files: Vec<PathBuf> = Self::collect_valid_files(root).collect();
            all_files.extend(files);
        }

        let _ = tx.send(LibraryRefreshProgress::Scanning { progress: 1 });

        // Second pass: Filter files
        let total_files = all_files.len();
        let mut new_files = Vec::new();

        for (i, path) in all_files.into_iter().enumerate() {
            if i % 100 == 0 || i == total_files - 1 {
                let progress = 5 + ((i * 10) / total_files.max(1)) as u8;
                let _ = tx.send(LibraryRefreshProgress::Scanning { progress });
            }

            let hash = calculate_signature(&path).unwrap_or(0);
            if !existing_hashes.remove(&hash) {
                new_files.push(path);
            }
        }

        let _ = tx.send(LibraryRefreshProgress::Scanning {
            progress: SCANNING_FINISHED,
        });

        // Phase 2: Processing song metadata
        let removed_ids = existing_hashes.into_iter().collect::<Vec<u64>>();
        let total_new = new_files.len();

        if !new_files.is_empty() {
            let _ = tx.send(LibraryRefreshProgress::Processing {
                progress: SCANNING_FINISHED,
                current: 0,
                total: total_new,
            });
            Self::insert_new_songs_with_progress(&mut self.db, new_files, tx)?;
        } else {
            let _ = tx.send(LibraryRefreshProgress::Processing {
                progress: PROCESSING_FINISHED,
                current: 0,
                total: 0,
            });
        }

        let _ = tx.send(LibraryRefreshProgress::UpdatingDatabase {
            progress: PROCESSING_FINISHED,
        });

        let total_removed = removed_ids.len();
        if !removed_ids.is_empty() {
            // Delete in batches for progress reporting
            for (i, chunk) in removed_ids.chunks(100).enumerate() {
                let progress = PROCESSING_FINISHED + ((i * 100 * 15) / total_removed.max(1)) as u8;
                let progress = progress.min(REMOVALS_FINISHED);
                let _ = tx.send(LibraryRefreshProgress::UpdatingDatabase { progress });
                self.db.delete_songs(chunk)?;
            }
        }

        let _ = tx.send(LibraryRefreshProgress::UpdatingDatabase {
            progress: REMOVALS_FINISHED,
        });

        // Phase 3: Collecting songs from database
        self.collect_songs()?;
        let _ = tx.send(LibraryRefreshProgress::UpdatingDatabase { progress: 90 });

        // Phase 4: Rebuilding library structures
        let _ = tx.send(LibraryRefreshProgress::Rebuilding { progress: 95 });
        self.build_albums()?;
        let _ = tx.send(LibraryRefreshProgress::Rebuilding { progress: 100 });

        Ok(())
    }

    fn insert_new_songs_with_progress(
        db: &mut Database,
        new_files: Vec<PathBuf>,
        tx: &Sender<LibraryRefreshProgress>,
    ) -> Result<()> {
        let total = new_files.len();
        let processed = AtomicUsize::new(0);
        let tx_clone = tx.clone();

        let songs: Vec<LongSong> = new_files
            .into_par_iter()
            .filter_map(|path| {
                let result = LongSong::build_song_lofty(&path).ok();

                let count = processed.fetch_add(1, Ordering::Relaxed) + 1;

                // Report progress periodically
                if count % 50 == 0 || count == total {
                    let progress = SCANNING_FINISHED + ((count * 30) / total.max(1)) as u8;
                    let _ = tx_clone.send(LibraryRefreshProgress::Processing {
                        progress,
                        current: count,
                        total,
                    });
                }

                result
            })
            .collect();

        let _ = tx.send(LibraryRefreshProgress::Processing {
            progress: 45,
            current: total,
            total,
        });

        let mut artist_cache = HashSet::new();
        let mut aa_binding = HashSet::new();

        for song in &songs {
            artist_cache.insert(song.get_artist());
            artist_cache.insert(song.album_artist.as_str());
            aa_binding.insert((song.album_artist.as_str(), song.get_album()));
        }

        db.insert_artists(&artist_cache)?;
        db.insert_albums(&aa_binding)?;
        db.insert_songs(&songs)?;

        Ok(())
    }
}
