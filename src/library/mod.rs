mod domain;
mod library;

pub use domain::LEGAL_EXTENSION;
pub use domain::{
    Album, FileType, LongSong, Playlist, PlaylistSong, SimpleSong, SongDatabase, SongInfo,
};
pub use library::Library;
