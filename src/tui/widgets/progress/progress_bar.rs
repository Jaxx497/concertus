use crate::{domain::SongInfo, ui_state::UiState};
use ratatui::{
    style::Stylize,
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
        let theme = state.theme_manager.get_display_theme(true);

        let np = state
            .get_now_playing()
            .expect("Expected a song to be playing. [Widget: Progress Bar]");
        let elapsed = state.get_playback_elapsed_f32();
        let duration = np.get_duration().as_secs_f32();

        // Prevent crash
        let ratio = (elapsed / duration).min(0.9999);

        let fg = theme.get_focused_color(ratio, elapsed);
        let bg = theme.get_inactive_color(ratio, elapsed, super::DEFAULT_AMP);

        let guage = LineGauge::default()
            .block(Block::new().bg(theme.bg_global).padding(Padding {
                left: 2,
                right: 3,
                top: (area.height / 2),
                bottom: 0,
            }))
            .filled_style(fg)
            .unfilled_style(bg)
            .filled_symbol(&theme.bar_active)
            .unfilled_symbol(&theme.bar_inactive)
            .label("")
            .ratio(ratio as f64);

        guage.render(area, buf);
    }
}
