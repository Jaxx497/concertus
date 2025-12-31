use super::{SimpleSong, SongInfo};
use crate::strip_win_prefix;
use crate::{domain::SongDatabase, get_readable_duration, Database};
use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use std::{sync::Arc, time::Duration};

pub struct QueueSong {
    pub meta: Arc<SimpleSong>,
    pub path: String,
}

impl QueueSong {
    pub fn from_simple_song(song: &Arc<SimpleSong>) -> Result<Arc<Self>> {
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
        Ok(self.path.clone())
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
    fn set_waveform_db(&self, wf: &[f32]) -> Result<()> {
        let mut db = Database::open()?;
        db.set_waveform(self.meta.id, wf)
    }
}
