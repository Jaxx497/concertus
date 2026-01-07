use crate::{
    player::PlaybackState,
    ui_state::{waveform::WaveformState, UiState},
    FFMPEG_AVAILABLE,
};

#[derive(Clone, Default, PartialEq, Eq)]
pub enum ProgressDisplay {
    Waveform,
    Oscilloscope,
    #[default]
    ProgressBar,
}

impl ProgressDisplay {
    pub fn from_str(s: &str) -> Self {
        match s {
            "oscilloscope" => Self::Oscilloscope,
            "waveform" => Self::Waveform,
            _ => Self::ProgressBar,
        }
    }
}

impl std::fmt::Display for ProgressDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProgressDisplay::Waveform => write!(f, "waveform"),
            ProgressDisplay::ProgressBar => write!(f, "progress_bar"),
            ProgressDisplay::Oscilloscope => write!(f, "oscilloscope"),
        }
    }
}

impl UiState {
    pub fn display_progress(&self) -> bool {
        self.metrics.get_state() != PlaybackState::Stopped || !self.queue_is_empty()
    }

    pub fn get_progress_display(&self) -> &ProgressDisplay {
        &self.progress_display
    }

    pub fn set_progress_display(&mut self, display: ProgressDisplay) {
        self.progress_display = match display {
            ProgressDisplay::Waveform => {
                if *FFMPEG_AVAILABLE {
                    match self.get_waveform_state() {
                        &WaveformState::Ready(_) => ProgressDisplay::Waveform,
                        _ => ProgressDisplay::default(),
                    }
                } else {
                    ProgressDisplay::default()
                }
            }
            ProgressDisplay::Oscilloscope => display,
            ProgressDisplay::ProgressBar => display,
        }
    }
}
