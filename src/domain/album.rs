use super::SimpleSong;
use std::sync::Arc;

#[derive(Default, Clone)]
pub struct Album {
    pub title: Arc<String>,
    pub artist: Arc<String>,
    pub year: Option<u32>,
    pub tracklist: Vec<Arc<SimpleSong>>,
}

impl Album {
    pub fn from_aa(title: &Arc<String>, artist: &Arc<String>) -> Self {
        Album {
            title: Arc::clone(&title),
            artist: Arc::clone(&artist),
            year: None,
            tracklist: Vec::new(),
        }
    }

    pub fn get_tracklist(&self) -> Vec<Arc<SimpleSong>> {
        self.tracklist.clone()
    }
}
