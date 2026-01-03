use crate::{
    get_readable_duration,
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

        let text_color = state.theme_manager.active.text_muted;

        let elapsed = state.playback.metrics.get_elapsed();
        let elapsed_str = get_readable_duration(elapsed, crate::DurationStyle::Compact);

        let duration = state.playback.now_playing.as_ref().unwrap().duration;
        let duration_str = get_readable_duration(duration, crate::DurationStyle::Compact);

        Text::from(elapsed_str)
            .fg(text_color)
            .left_aligned()
            .render(Rect::new(3, y_pos, DUR_WIDTH, 1), buf);

        Text::from(duration_str)
            .fg(text_color)
            .right_aligned()
            .render(Rect::new(x_pos, y_pos, DUR_WIDTH, 1), buf);
    }
}
