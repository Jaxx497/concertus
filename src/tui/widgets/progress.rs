use crate::{tui::widgets::Waveform, ui_state::UiState};
use ratatui::widgets::StatefulWidget;

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
