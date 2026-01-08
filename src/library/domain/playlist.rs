use crate::{get_readable_duration, library::SongInfo};

use super::SimpleSong;
use std::{sync::Arc, time::Duration};

pub struct Playlist {
    pub id: i64,
    pub name: String,
    pub tracklist: Vec<PlaylistSong>,
    length: Duration,
}

impl Playlist {
    pub fn new(id: i64, name: String, tracklist: Vec<PlaylistSong>) -> Self {
        let length: Duration = tracklist.iter().map(|s| s.get_duration()).sum();

        Playlist {
            id,
            name,
            tracklist,
            length,
        }
    }

    pub fn get_tracklist(&self) -> Vec<Arc<SimpleSong>> {
        self.tracklist
            .iter()
            .map(|s| Arc::clone(&s.song))
            .collect::<Vec<_>>()
    }

    pub fn get_total_length(&self) -> Duration {
        self.length
    }

    pub fn len(&self) -> usize {
        self.tracklist.len()
    }

    pub fn update_length(&mut self) {
        self.length = self.tracklist.iter().map(|s| s.get_duration()).sum();
    }
}

pub struct PlaylistSong {
    pub id: i64,
    pub song: Arc<SimpleSong>,
}

impl SongInfo for PlaylistSong {
    fn get_id(&self) -> u64 {
        self.song.id
    }

    fn get_title(&self) -> &str {
        &self.song.title
    }

    fn get_artist(&self) -> &str {
        &self.song.artist
    }

    fn get_album(&self) -> &str {
        &self.song.album
    }

    fn get_duration(&self) -> Duration {
        self.song.duration
    }

    fn get_duration_f32(&self) -> f32 {
        self.song.duration.as_secs_f32()
    }

    fn get_duration_str(&self) -> String {
        get_readable_duration(self.song.duration, crate::DurationStyle::Compact)
    }
}
