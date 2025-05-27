use super::{SimpleSong, SongInfo};
use crate::get_readable_duration;
use std::{sync::Arc, time::Duration};

pub struct QueueSong {
    pub meta: Arc<SimpleSong>,
    pub path: String,
}

impl SongInfo for QueueSong {
    fn get_id(&self) -> u64 {
        self.meta.id
    }

    fn get_title(&self) -> &str {
        &self.meta.title
    }

    fn get_artist(&self) -> &str {
        &self.meta.artist
    }

    fn get_album(&self) -> &str {
        &self.meta.album
    }

    fn get_duration(&self) -> Duration {
        self.meta.duration
    }

    fn get_duration_f32(&self) -> f32 {
        self.meta.duration.as_secs_f32()
    }

    fn get_duration_str(&self) -> String {
        get_readable_duration(self.meta.duration, crate::DurationStyle::Compact)
    }
}
