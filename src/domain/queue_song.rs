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
