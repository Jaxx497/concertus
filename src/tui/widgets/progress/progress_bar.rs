use crate::{domain::SongInfo, tui::widgets::DUR_WIDTH, ui_state::UiState};
use ratatui::{
    layout::Rect,
    style::{Color, Stylize},
    symbols::line,
    text::Text,
    widgets::{Block, LineGauge, Padding, StatefulWidget, Widget},
};

pub struct ProgressBar;

impl StatefulWidget for ProgressBar {
    type State = UiState;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let np = state
            .get_now_playing()
            .expect("Expected a song to be playing. [Widget: Progress Bar]");
        let elapsed = state.get_playback_elapsed();
        let duration = np.get_duration().as_secs_f32();
        let progress_raw = elapsed.as_secs_f32() / duration;

        // The program will crash if this hit's 1.0
        let ratio = match progress_raw {
            i if i < 1.0 => i,
            _ => 0.0,
        };

        let player_state = state.playback.player_state.lock().unwrap();
        let elapsed_str = player_state.elapsed_display.as_str();
        let duration_str = player_state.duration_display.as_str();

        let x_duration = area.width - 8;
        let y = buf.area().height
            - match area.height {
                0 => 1,
                _ => area.height / 2 + 1,
            };

        Text::from(elapsed_str)
            .fg(Color::DarkGray)
            .right_aligned()
            .render(Rect::new(2, y, DUR_WIDTH, 1), buf);

        Text::from(duration_str)
            .fg(Color::DarkGray)
            .right_aligned()
            .render(Rect::new(x_duration, y, DUR_WIDTH, 1), buf);

        let guage = LineGauge::default()
            .block(Block::new().bg(state.theme.bg_unfocused).padding(Padding {
                left: 10,
                right: 10,
                top: 2,
                bottom: 0,
            }))
            .filled_style(state.theme.text_secondary)
            .line_set(line::THICK)
            .label("")
            .ratio(ratio as f64);

        guage.render(area, buf);
    }
}
