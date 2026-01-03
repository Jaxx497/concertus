// THIS WILL BE A PAIN IN THE ASS, SHOULD PROBABLY WAIT UNTIL WE FIGURE OUT THE QUEUE SITUATION

use crate::domain::{SimpleSong, SongDatabase};

pub(super) struct ConcertusTrack<I> {
    id: I,
    path: String,
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
            path: song.get_path()?,
        })
    }
}

