use super::PAUSE_ICON;
use crate::{domain::SongInfo, get_readable_duration, ui_state::UiState, DurationStyle};
use ratatui::{
    layout::Alignment,
    style::{Color, Stylize},
    symbols,
    text::{Line, Span},
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
        // let theme = &state.get_theme(&Pane::TrackList);

        // The program will crash if this hit's 1.0
        let ratio = match progress_raw {
            i if i < 1.0 => i,
            _ => 0.0,
        };

        let is_paused = state.is_paused().then(|| PAUSE_ICON).unwrap_or("");

        // BUG: This label creation MAY cause the search
        // cursor to flicker indcredibly quickly
        // specifically in the Windows terminal/cmd
        let label = match state.is_not_playing() {
            true => "0:00.00 / 0:00".into(),
            false => {
                format!(
                    "{:1} {} / {}", // :1 Prevents shift in widget when pause icon appears
                    is_paused,
                    get_readable_duration(elapsed, DurationStyle::Compact),
                    get_readable_duration(np.get_duration(), DurationStyle::Compact),
                )
            }
        };

        let playing_title = Line::from_iter([
            Span::from(np.get_title()).fg(Color::Red),
            Span::from(" ✧ ").fg(Color::DarkGray),
            Span::from(np.get_artist()).fg(Color::Gray),
        ]);

        let guage = LineGauge::default()
            .block(
                Block::new()
                    .title_top(playing_title.alignment(Alignment::Center))
                    .padding(Padding {
                        left: 10,
                        right: 10,
                        top: 2,
                        bottom: 0,
                    }),
            )
            .filled_style(ratatui::style::Color::Magenta)
            .line_set(symbols::line::THICK)
            .label(label)
            .ratio(ratio as f64);

        guage.render(area, buf);
    }
}
