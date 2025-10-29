use super::{FileType, SongInfo};
use crate::{
    calculate_signature, database::Database, get_readable_duration, normalize_metadata_str as nms,
};
use anyhow::{Result, bail};
use lofty::{
    file::{AudioFile, TaggedFileExt},
    read_from_path,
    tag::{Accessor, ItemKey},
};

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

#[derive(Default)]
pub struct LongSong {
    pub(crate) id: u64,
    pub(crate) title: String,
    pub(crate) year: Option<u32>,
    pub(crate) artist: Arc<String>,
    pub(crate) album_artist: Arc<String>,
    pub(crate) album: Arc<String>,
    pub(crate) track_no: Option<u32>,
    pub(crate) disc_no: Option<u32>,
    pub(crate) duration: Duration,
    pub(crate) channels: Option<u8>,
    pub(crate) bit_rate: Option<u32>,
    pub(crate) sample_rate: Option<u32>,
    pub(crate) filetype: FileType,
    pub(crate) path: PathBuf,
}

impl LongSong {
    pub fn new(path: PathBuf) -> Self {
        LongSong {
            path,
            ..Default::default()
        }
    }

    pub fn build_song_lofty<P: AsRef<Path>>(path_raw: P) -> Result<LongSong> {
        let path = path_raw.as_ref();
        let mut song_info = LongSong::new(PathBuf::from(path));

        song_info.id = calculate_signature(path)?;

        song_info.filetype = match path.extension() {
            Some(n) => FileType::from(n.to_str().unwrap()),
            None => bail!("Unsupported extension: {:?}", path.extension()),
        };

        let tagged_file = read_from_path(path)?;
        let properties = tagged_file.properties();

        song_info.duration = properties.duration();
        song_info.channels = properties.channels();
        song_info.sample_rate = properties.sample_rate();
        song_info.bit_rate = properties.audio_bitrate();

        if let Some(tag) = tagged_file.primary_tag() {
            song_info.title = tag
                .title()
                .map(|s| nms(&s))
                .filter(|s| !s.is_empty())
                .unwrap_or(
                    path.file_stem()
                        .map(|stem| stem.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                );

            song_info.album = Arc::new(tag.album().map(|s| nms(&s)).unwrap_or_default());

            let artist = tag.artist().map(|s| nms(&s)).unwrap_or_default();

            let album_artist = tag
                .get_string(&ItemKey::AlbumArtist)
                .map(|s| nms(&s))
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| artist.to_string());

            song_info.artist = Arc::new(artist);
            song_info.album_artist = Arc::new(album_artist);

            //
            song_info.year = tag.year().or_else(|| {
                tag.get_string(&ItemKey::Year)
                    .and_then(|s| {
                        nms(&s)
                            .split_once('-')
                            .map(|(y, _)| y.to_string())
                            .or_else(|| Some(s.to_string()))
                    })
                    .and_then(|s| s.parse::<u32>().ok())
            });

            song_info.track_no = tag.track();
            song_info.disc_no = tag.disk();
        }

        Ok(song_info)
    }

    pub fn get_path(&self, db: &mut Database) -> Result<String> {
        db.get_song_path(self.id)
    }
}

impl SongInfo for LongSong {
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
