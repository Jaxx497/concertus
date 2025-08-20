use super::SimpleSong;
use std::sync::Arc;

#[derive(Clone)]
pub struct Playlist {
    pub id: u64,
    pub name: String,
    // pub tracks: Vec<Arc<SimpleSong>>,
}

impl Playlist {
    pub fn new(id: u64, name: String) -> Self {
        Playlist {
            id,
            name,
            // tracks: Vec::new(),
        }
    }
}
