mod app;
pub use app::Concertus;

pub enum LibraryRefreshProgress {
    Scanning {
        progress: u8,
    },
    Processing {
        progress: u8,
        current: usize,
        total: usize,
    },
    UpdatingDatabase {
        progress: u8,
    },
    Rebuilding {
        progress: u8,
    },
    Complete(crate::Library),
    Error(String),
}
