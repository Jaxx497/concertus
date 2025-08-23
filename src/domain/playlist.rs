use super::SimpleSong;
use std::sync::Arc;

#[derive(Clone)]
pub struct Playlist {
    pub id: i64,
    pub name: String,
    // pub tracks: Vec<Arc<SimpleSong>>,
}

impl Playlist {
    pub fn new(id: i64, name: String) -> Self {
        Playlist {
            id,
            name,
            // tracks: Vec::new(),
        }
    }
}
