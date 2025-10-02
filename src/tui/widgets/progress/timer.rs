use ratatui::{
    layout::Rect,
    style::{Color, Stylize},
    text::Text,
    widgets::{StatefulWidget, Widget},
};

use crate::{
    tui::widgets::DUR_WIDTH,
    ui_state::{Mode, ProgressDisplay, UiState},
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
        let state_mode = state.get_mode();

        let x_pos = area.width - 8;
        let mut y_pos = match matches!(state_mode, Mode::Fullscreen) {
            false => buf.area().height - area.height / 2,
            true => area.height - 1,
        };

        if let ProgressDisplay::Waveform = state.get_progress_display()
            && !matches!(state_mode, Mode::Fullscreen)
        {
            y_pos -= 1
        }

        let player_state = state.playback.player_state.lock().unwrap();
        {
            let elapsed_str = player_state.elapsed_display.as_str();
            let duration_str = player_state.duration_display.as_str();

            Text::from(elapsed_str)
                .fg(Color::DarkGray)
                .right_aligned()
                .render(Rect::new(2, y_pos, DUR_WIDTH, 1), buf);

            Text::from(duration_str)
                .fg(Color::DarkGray)
                .right_aligned()
                .render(Rect::new(x_pos, y_pos, DUR_WIDTH, 1), buf);
        }
        drop(player_state);
    }
}
