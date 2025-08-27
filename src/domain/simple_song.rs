use super::{FileType, SongInfo};
use crate::{get_readable_duration, Database};
use std::{sync::Arc, time::Duration};

#[derive(Default, Hash, Eq, PartialEq)]
pub struct SimpleSong {
    pub(crate) id: u64,
    pub(crate) title: String,
    pub(crate) artist: Arc<String>,
    pub(crate) year: Option<u32>,
    pub(crate) album: Arc<String>,
    pub(crate) album_artist: Arc<String>,
    pub(crate) track_no: Option<u32>,
    pub(crate) disc_no: Option<u32>,
    pub(crate) duration: Duration,
    pub(crate) filetype: FileType,
}

impl SimpleSong {
    pub fn get_path(&self, db: &mut Database) -> anyhow::Result<String> {
        db.get_path(self.id)
    }
}

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

//     fn hash(&self) -> u64 {
//         self.id
//     }
// }
