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
