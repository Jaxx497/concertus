use super::{FileType, SongInfo};
use crate::{Database, get_readable_duration};
use anyhow::Result;
use std::{sync::Arc, time::Duration};

#[derive(Default, Hash, Eq, PartialEq)]
pub struct SimpleSong {
    pub(crate) id: u64,
    pub(crate) title: String,
    pub(crate) artist: Arc<String>,
    pub(crate) year: Option<u32>,
    pub(crate) album: Arc<String>,
    pub(crate) album_id: i64,
    pub(crate) album_artist: Arc<String>,
    pub(crate) track_no: Option<u32>,
    pub(crate) disc_no: Option<u32>,
    pub(crate) duration: Duration,
    pub(crate) filetype: FileType,
}

/// DATABASE RELATED METHODS
impl super::SongDatabase for SimpleSong {
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
    fn set_waveform_db(&self, wf: &[f32]) -> Result<()> {
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
