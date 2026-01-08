mod backend;
mod backend_rodio;
mod core;
mod handle;
mod metrics;
mod track;

pub use crate::player::track::ConcertusTrack;
use backend::ConcertusBackend;
pub use handle::PlayerHandle;
pub use metrics::PlaybackMetrics;

pub(crate) const OSCILLO_BUFFER_CAPACITY: usize = 2048;

pub enum PlayerEvent {
    TrackStarted((ConcertusTrack, bool)),
    PlaybackStopped,
    Error(String),
}

pub enum PlayerCommand {
    Play(ConcertusTrack),
    SetNext(Option<ConcertusTrack>),
    ClearNext,
    TogglePlayback,
    Stop,
    SeekForward(u64),
    SeekBack(u64),
}

#[derive(PartialEq, Eq)]
#[repr(u8)]
pub enum PlaybackState {
    Stopped = 0,
    Playing = 1,
    Paused = 2,
}

impl From<PlaybackState> for u8 {
    fn from(state: PlaybackState) -> u8 {
        state as u8
    }
}

impl TryFrom<u8> for PlaybackState {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(PlaybackState::Stopped),
            1 => Ok(PlaybackState::Playing),
            2 => Ok(PlaybackState::Paused),
            _ => Err(()),
        }
    }
}
