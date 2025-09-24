mod progress_bar;
mod waveform;
use ratatui::widgets::StatefulWidget;

use crate::{
    tui::widgets::progress::{progress_bar::ProgressBar, waveform::Waveform},
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
            match (state.waveform_is_valid(), &state.which_display_style()) {
                (true, ProgressDisplay::Waveform) => Waveform.render(area, buf, state),
                _ => ProgressBar.render(area, buf, state),
            }
        }
    }
}
