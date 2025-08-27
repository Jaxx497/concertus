use super::{FileType, SongInfo};
use crate::{calculate_signature, database::Database, get_readable_duration};
use anyhow::{anyhow, Context, Result};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use symphonia::core::{
    io::MediaSourceStream,
    meta::{StandardTagKey, Value},
    probe::Hint,
};

#[derive(Default)]
pub struct LongSong {
    pub(crate) id: u64,
    pub(crate) title: String,
    pub(crate) artist: Arc<String>,
    pub(crate) album_artist: Arc<String>,
    pub(crate) album: Arc<String>,
    pub(crate) track_no: Option<u32>,
    pub(crate) disc_no: Option<u32>,
    pub(crate) duration: Duration,
    pub(crate) sample_rate: u32,
    pub(crate) year: Option<u32>,
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

    pub fn build_song_symphonia<P: AsRef<Path>>(path_raw: P) -> Result<LongSong> {
        let path = path_raw.as_ref();

        let extension = path.extension();
        let format = match extension {
            Some(n) => FileType::from(n.to_str().unwrap()),
            None => return Err(anyhow!("Unsuppored extension: {:?}", path.extension())),
        };

        let src = std::fs::File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(src), Default::default());
        let mut hint = Hint::new();

        if let Some(ext) = extension {
            if let Some(ext_str) = ext.to_str() {
                hint.with_extension(ext_str);
            }
        }

        let mut probed = symphonia::default::get_probe().format(
            &hint,
            mss,
            &Default::default(),
            &Default::default(),
        )?;

        let mut song_info = LongSong::new(PathBuf::from(path));

        song_info.filetype = format;
        song_info.id = calculate_signature(path)?;

        let track = probed.format.default_track().context("No default track")?;

        if let Some(n_frames) = track.codec_params.n_frames {
            let sample_rate = track
                .codec_params
                .sample_rate
                .context("Sample rate is not specified")?;

            let duration_raw = Duration::from_secs_f32(n_frames as f32 / sample_rate as f32);

            song_info.sample_rate = sample_rate;
            song_info.duration = duration_raw;
        }

        let metadata = match probed.metadata.get() {
            Some(m) => m,
            None => probed.format.metadata(),
        };

        let tags = metadata
            .current()
            .context("Could not get current metadata from file!")?
            .tags();

        for tag in tags {
            if let Some(key) = tag.std_key {
                song_info.match_tags(key, &tag.value);
            }
        }

        if !song_info.artist.is_empty() && song_info.album_artist.is_empty() {
            song_info.album_artist = Arc::clone(&song_info.artist);
        }

        if song_info.title.is_empty() {
            song_info.title = path
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default()
        }

        if song_info.filetype == FileType::M4A {
            let tag = mp4ameta::Tag::read_from_path(path).unwrap();
            song_info.disc_no = tag.disc_number().map(u32::from);
        }

        Ok(song_info)
    }

    fn match_tags(&mut self, key: StandardTagKey, value: &Value) {
        match key {
            StandardTagKey::TrackTitle => self.title = value.to_string(),
            StandardTagKey::Album => self.album = Arc::new(value.to_string()),
            StandardTagKey::Artist => self.artist = Arc::new(value.to_string()),
            StandardTagKey::AlbumArtist => self.album_artist = Arc::new(value.to_string()),
            StandardTagKey::Date => {
                self.year = value
                    .to_string()
                    .split_once('-')
                    .map(|(year, _)| year)
                    .unwrap_or(&value.to_string())
                    .parse::<u32>()
                    .ok()
            }
            StandardTagKey::TrackNumber => {
                self.track_no = value
                    .to_string()
                    .split_once('/')
                    .map(|(num, _)| num)
                    .unwrap_or(&value.to_string())
                    .parse::<u32>()
                    .ok()
            }
            StandardTagKey::DiscNumber => {
                self.disc_no = value
                    .to_string()
                    .split_once('/')
                    .map(|(num, _)| num)
                    .unwrap_or(&value.to_string())
                    .parse::<u32>()
                    .ok()
            }
            _ => {}
        }
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
