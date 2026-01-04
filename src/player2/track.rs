use std::path::PathBuf;

use crate::domain::{SimpleSong, SongDatabase};

#[derive(Clone)]
pub struct ConcertusTrack<I> {
    id: I,
    path: PathBuf,
}

impl<I: PartialEq> PartialEq for ConcertusTrack<I> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl TryFrom<&SimpleSong> for ConcertusTrack<u64> {
    type Error = anyhow::Error;

    fn try_from(song: &SimpleSong) -> Result<Self, Self::Error> {
        Ok(Self {
            id: song.id,
            path: PathBuf::from(song.get_path()?),
        })
    }
}

impl ConcertusTrack<u64> {
    pub fn get_id(&self) -> u64 {
        self.id
    }

    pub fn get_path(&self) -> &PathBuf {
        &self.path
    }
}
