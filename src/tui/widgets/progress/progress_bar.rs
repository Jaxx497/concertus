use crate::{domain::SongInfo, ui_state::UiState};
use ratatui::{
    style::{Color, Stylize},
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
        let elapsed = state.get_playback_elapsed().as_secs_f32();
        let duration = np.get_duration().as_secs_f32();
        let progress_raw = elapsed / duration;

        // The program will crash if this hit's 1.0
        let ratio = match progress_raw {
            i if i < 1.0 => i,
            _ => 0.0,
        };

        let guage = LineGauge::default()
            .block(
                Block::new()
                    .bg(state.theme_manager.active.surface_global)
                    .padding(Padding {
                        left: 1,
                        right: 2,
                        top: (area.height / 2),
                        bottom: 0,
                    }),
            )
            .filled_style(get_vibrant_color(ratio, elapsed))
            .unfilled_style(state.theme_manager.active.text_muted)
            .line_set(line::THICK)
            .label("")
            .ratio(ratio as f64);

        guage.render(area, buf);
    }
}

fn get_vibrant_color(position: f32, time: f32) -> Color {
    let hue = (position * 360.0 + time * 30.0) % 360.0;
    let saturation = 1.0;
    let value = 0.9;

    super::hsv_to_rgb(hue, saturation, value)
}
