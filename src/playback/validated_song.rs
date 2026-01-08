use crate::{
    Database, get_readable_duration,
    library::{SimpleSong, SongDatabase, SongInfo},
};
use anyhow::Result;
use std::{path::PathBuf, sync::Arc, time::Duration};

pub struct ValidatedSong {
    pub meta: Arc<SimpleSong>,
    pub path: String,
}

impl ValidatedSong {
    pub fn new(song: &Arc<SimpleSong>) -> Result<Arc<Self>> {
        let path = song.get_path()?;

        std::fs::metadata(&path)?;

        Ok(Arc::new(Self {
            meta: Arc::clone(&song),
            path,
        }))
    }

    pub fn id(&self) -> u64 {
        self.meta.get_id()
    }

    pub fn path_str(&self) -> String {
        self.path.clone()
    }

    pub fn path(&self) -> PathBuf {
        PathBuf::from(&self.path)
    }
}

impl SongInfo for ValidatedSong {
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

impl SongDatabase for ValidatedSong {
    /// Returns the path of a song as a String
    fn get_path(&self) -> Result<String> {
        let mut db = Database::open()?;
        db.get_song_path(self.id())
    }

    /// Update the play_count of the song
    fn update_play_count(&self) -> Result<()> {
        let mut db = Database::open()?;
        db.update_play_count(self.id())
    }

    /// Retrieve the waveform of a song
    /// returns Result<Vec<f32>>
    fn get_waveform(&self) -> Result<Vec<f32>> {
        let mut db = Database::open()?;
        db.get_waveform(self.id())
    }

    /// Store the waveform of a song in the databse
    fn set_waveform_db(&self, wf: &[f32]) -> Result<()> {
        let mut db = Database::open()?;
        db.set_waveform(self.id(), wf)
    }
}
