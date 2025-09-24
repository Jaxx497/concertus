use anyhow::anyhow;

use crate::{
    domain::smooth_waveform, key_handler::MoveDirection, player::PlaybackState, ui_state::UiState,
};

pub enum ProgressDisplay {
    Waveform,
    ProgressBar,
}

pub struct PlaybackView {
    pub waveform_raw: Vec<f32>,
    pub waveform_smooth: Vec<f32>,
    pub waveform_smoothing: f32,
    waveform_valid: bool,
    pub progress_display: ProgressDisplay,
}

impl PlaybackView {
    pub fn new() -> Self {
        PlaybackView {
            waveform_raw: Vec::new(),
            waveform_smooth: Vec::new(),
            waveform_smoothing: 1.0,
            waveform_valid: true,
            progress_display: ProgressDisplay::Waveform,
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
        self.clear_waveform();
        self.playback_view.waveform_valid = false
    }

    pub fn waveform_is_valid(&self) -> bool {
        self.playback_view.waveform_valid
    }

    pub fn which_display_style(&self) -> &ProgressDisplay {
        if !&self.playback_view.waveform_valid {
            return &ProgressDisplay::ProgressBar;
        } else {
            &self.playback_view.progress_display
        }
    }

    pub fn toggle_progress_display(&mut self) {
        match self.playback_view.progress_display {
            ProgressDisplay::Waveform => {
                self.playback_view.progress_display = ProgressDisplay::ProgressBar
            }
            ProgressDisplay::ProgressBar => {
                if !self.playback_view.waveform_valid {
                    self.set_error(anyhow!("Invalid Waveform!\n"));
                }
                self.playback_view.progress_display = ProgressDisplay::Waveform
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
