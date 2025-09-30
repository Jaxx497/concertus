mod oscilloscope;
mod progress_bar;
mod timer;
mod waveform;
use ratatui::widgets::StatefulWidget;

use crate::{
    tui::widgets::progress::{
        oscilloscope::Oscilloscope, progress_bar::ProgressBar, timer::Timer, waveform::Waveform,
    },
    ui_state::{ProgressDisplay, UiState},
};

pub struct Progress;
impl StatefulWidget for Progress {
    type State = UiState;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        if state.get_now_playing().is_some() {
            match &state.get_progress_display() {
                ProgressDisplay::ProgressBar => ProgressBar.render(area, buf, state),
                ProgressDisplay::Waveform => match state.waveform_is_valid() {
                    true => Waveform.render(area, buf, state),
                    false => Oscilloscope.render(area, buf, state),
                },
                ProgressDisplay::Oscilloscope => Oscilloscope.render(area, buf, state),
            }
            Timer.render(area, buf, state);
        }
    }
}
