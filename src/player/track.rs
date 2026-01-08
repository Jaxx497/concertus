use std::path::PathBuf;

use crate::{
    library::{SimpleSong, SongDatabase},
    playback::ValidatedSong,
};

#[derive(Clone)]
pub struct ConcertusTrack {
    id: u64,
    path: PathBuf,
}

impl PartialEq for ConcertusTrack {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl TryFrom<&SimpleSong> for ConcertusTrack {
    type Error = anyhow::Error;

    fn try_from(song: &SimpleSong) -> Result<Self, Self::Error> {
        Ok(Self {
            id: song.id,
            path: PathBuf::from(song.get_path()?),
        })
    }
}

impl From<&ValidatedSong> for ConcertusTrack {
    fn from(song: &ValidatedSong) -> Self {
        ConcertusTrack {
            id: song.id(),
            path: song.path(),
        }
    }
}

impl ConcertusTrack {
    pub fn new(id: u64, path: PathBuf) -> Self {
        ConcertusTrack { id, path }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}
