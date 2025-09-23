mod album;
mod filetype;
mod long_song;
mod playlist;
mod queue_song;
mod simple_song;
mod waveform;

pub use album::Album;
pub use filetype::FileType;
pub use long_song::LongSong;
pub use playlist::Playlist;
pub use playlist::PlaylistSong;
pub use queue_song::QueueSong;
pub use simple_song::SimpleSong;
pub use waveform::{generate_waveform, smooth_waveform};

pub trait SongInfo {
    fn get_id(&self) -> u64;
    fn get_title(&self) -> &str;
    fn get_artist(&self) -> &str;
    fn get_album(&self) -> &str;
    fn get_duration(&self) -> std::time::Duration;
    fn get_duration_f32(&self) -> f32;
    fn get_duration_str(&self) -> String;
}

pub trait SongDatabase {
    fn get_path(&self) -> anyhow::Result<String>;
    fn update_play_count(&self) -> anyhow::Result<()>;
    fn get_waveform(&self) -> anyhow::Result<Vec<f32>>;
    fn set_waveform_db(&self, wf: &[f32]) -> anyhow::Result<()>;
}
