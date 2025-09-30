use crate::{domain::SongInfo, ui_state::UiState};
use ratatui::{
    style::Stylize,
    symbols::line,
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

        let guage = LineGauge::default()
            .block(Block::new().bg(state.theme.bg_unfocused).padding(Padding {
                left: 2,
                right: 2,
                top: 2,
                bottom: 0,
            }))
            .filled_style(state.theme.progress_complete)
            .unfilled_style(state.theme.progress_incomplete)
            .line_set(line::THICK)
            .label("")
            .ratio(ratio as f64);

        guage.render(area, buf);
    }
}
