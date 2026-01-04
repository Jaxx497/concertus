mod session;
mod validated_song;

pub const HISTORY_CAPACITY: usize = 50;

pub use session::PlaybackSession;
pub use validated_song::ValidatedSong;

use std::sync::Arc;

pub enum QueueDelta {
    HeadUnchanged,
    HeadChanged {
        prev: Option<Arc<ValidatedSong>>,
        curr: Option<Arc<ValidatedSong>>,
    },
}
