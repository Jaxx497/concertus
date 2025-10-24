use crate::{
    tui::widgets::DUR_WIDTH,
    ui_state::{ProgressDisplay, UiState},
};
use ratatui::{
    layout::Rect,
    style::Stylize,
    text::Text,
    widgets::{StatefulWidget, Widget},
};

pub struct Timer;
impl StatefulWidget for Timer {
    type State = UiState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let x_pos = area.width - 8;
        let y_pos = match state.get_progress_display() {
            ProgressDisplay::Waveform => area.y + (area.height / 2),
            _ => area.y + area.height.saturating_sub(1),
        };

        let text_color = state.theme_manager.active.text.1;
        let player_state = state.playback.player_state.lock().unwrap();
        {
            let elapsed_str = player_state.elapsed_display.as_str();
            let duration_str = player_state.duration_display.as_str();

            Text::from(elapsed_str)
                .fg(text_color)
                .right_aligned()
                .render(Rect::new(2, y_pos, DUR_WIDTH, 1), buf);

            Text::from(duration_str)
                .fg(text_color)
                .right_aligned()
                .render(Rect::new(x_pos, y_pos, DUR_WIDTH, 1), buf);
        }
    }
}
