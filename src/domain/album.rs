use super::SimpleSong;
use std::sync::Arc;

#[derive(Default, Clone)]
pub struct Album {
    pub id: i64,
    pub title: Arc<String>,
    pub artist: Arc<String>,
    pub year: Option<u32>,
    pub tracklist: Arc<[Arc<SimpleSong>]>,
}

impl Album {
    pub fn from_aa(id: i64, title: Arc<String>, artist: Arc<String>) -> Self {
        Album {
            id,

            title,
            artist,
            year: None,
            tracklist: Arc::new([]),
        }
    }

    pub fn get_tracklist(&self) -> Vec<Arc<SimpleSong>> {
        self.tracklist.to_vec()
    }
}
