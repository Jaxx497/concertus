use anyhow::anyhow;

use crate::{
    domain::smooth_waveform, key_handler::MoveDirection, player::PlaybackState, ui_state::UiState,
};

#[derive(PartialEq, Eq)]
pub enum ProgressDisplay {
    Waveform,
    ProgressBar,
    Oscilloscope,
}

impl ProgressDisplay {
    pub fn from_str(s: &str) -> Self {
        match s {
            "progress_bar" => Self::ProgressBar,
            "oscilloscope" => Self::Oscilloscope,
            _ => Self::Waveform,
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

pub struct PlaybackView {
    pub waveform_raw: Vec<f32>,
    pub waveform_smooth: Vec<f32>,
    pub waveform_smoothing: f32,
    waveform_valid: bool,
    progress_display: ProgressDisplay,
}

impl PlaybackView {
    pub fn new() -> Self {
        PlaybackView {
            waveform_raw: Vec::new(),
            waveform_smooth: Vec::new(),
            waveform_smoothing: 1.0,
            waveform_valid: true,
            progress_display: ProgressDisplay::Oscilloscope,
        }
    }
}

impl UiState {
    pub fn get_waveform_visual(&self) -> &[f32] {
        self.playback_view.waveform_smooth.as_slice()
    }

    pub fn set_waveform_visual(&mut self, wf: Vec<f32>) {
        self.playback_view.waveform_raw = wf;
        self.playback_view.smooth_waveform();
    }

    pub fn clear_waveform(&mut self) {
        self.playback_view.waveform_raw.clear();
        self.playback_view.waveform_smooth.clear();
    }

    pub fn display_waveform(&self) -> bool {
        let state = self.playback.player_state.lock().unwrap();
        state.state != PlaybackState::Stopped || !self.queue_is_empty()
    }

    pub fn set_waveform_valid(&mut self) {
        self.playback_view.waveform_valid = true
    }

    pub fn set_waveform_invalid(&mut self) {
        self.playback_view.waveform_valid = false;
        self.clear_waveform();
    }

    pub fn waveform_is_valid(&self) -> bool {
        self.playback_view.waveform_valid
    }

    pub fn get_progress_display(&self) -> &ProgressDisplay {
        &self.playback_view.progress_display
    }

    pub fn set_progress_display(&mut self, display: ProgressDisplay) {
        self.playback_view.progress_display = match display {
            ProgressDisplay::Waveform => match !self.waveform_is_valid() {
                true => {
                    self.set_error(anyhow!("Invalid waveform! \nFallback to Oscilloscope"));
                    ProgressDisplay::Oscilloscope
                }
                false => display,
            },
            ProgressDisplay::Oscilloscope => display,
            ProgressDisplay::ProgressBar => display,
        }
    }

    pub fn get_oscilloscope_data(&self) -> Vec<f32> {
        match self.playback.player_state.lock() {
            Ok(state) => state.oscilloscope_buffer.iter().copied().collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn next_progress_display(&mut self) {
        self.playback_view.progress_display = match self.playback_view.progress_display {
            ProgressDisplay::Waveform => ProgressDisplay::Oscilloscope,
            ProgressDisplay::Oscilloscope => ProgressDisplay::ProgressBar,
            ProgressDisplay::ProgressBar => {
                if !self.playback_view.waveform_valid {
                    self.set_error(anyhow!("Invalid Waveform!\n"));
                    ProgressDisplay::Oscilloscope
                } else {
                    ProgressDisplay::Waveform
                }
            }
        }
    }
}

static WAVEFORM_STEP: f32 = 0.5;
impl PlaybackView {
    pub fn increment_smoothness(&mut self, direction: MoveDirection) {
        match direction {
            MoveDirection::Up => {
                if self.waveform_smoothing < 3.9 {
                    self.waveform_smoothing += WAVEFORM_STEP;
                    self.smooth_waveform();
                }
            }
            MoveDirection::Down => {
                if self.waveform_smoothing > 0.1 {
                    self.waveform_smoothing -= WAVEFORM_STEP;
                    self.smooth_waveform();
                }
            }
        }
    }

    pub fn smooth_waveform(&mut self) {
        self.waveform_smooth = self.waveform_raw.clone();
        smooth_waveform(&mut self.waveform_smooth, self.waveform_smoothing);
    }
}
