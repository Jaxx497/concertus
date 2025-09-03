mod waveform;
use ratatui::widgets::StatefulWidget;

use crate::{tui::widgets::progress::waveform::Waveform, ui_state::UiState};

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
            Waveform.render(area, buf, state);
        }
    }
}
