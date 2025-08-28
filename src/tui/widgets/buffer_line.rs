use crate::{
    domain::SongInfo,
    tui::widgets::PAUSE_ICON,
    ui_state::{DisplayTheme, UiState, GOLD_FADED},
};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Stylize,
    text::{Line, Span},
    widgets::{StatefulWidget, Widget},
};

pub struct BufferLine;

impl StatefulWidget for BufferLine {
    type State = UiState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let theme = state.get_theme(state.get_pane());

        let separator = match state.is_paused() {
            true => Span::from(format!(" {PAUSE_ICON} ")).fg(theme.text_focused),
            false => Span::from(" ✧ ").fg(theme.text_faded),
        };

        let playing_title = match state.get_now_playing() {
            Some(s) => Line::from_iter([
                Span::from(s.get_title().to_string()).fg(theme.text_secondary),
                separator,
                Span::from(s.get_artist().to_string()).fg(theme.text_faded),
            ])
            .centered(),
            None => "".into(),
        };

        let bulk_selection = match state.get_bulk_sel().len() {
            0 => "".into(),
            x => format!("  {x} songs selected")
                .fg(theme.text_faded)
                .into_left_aligned_line(),
        };

        let [left, center, right] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(40),
                Constraint::Percentage(30),
            ])
            .areas(area);

        bulk_selection.render(left, buf);
        playing_title.render(center, buf);
        queue_display(state, &theme).render(right, buf);
    }
}

fn queue_display(state: &UiState, theme: &DisplayTheme) -> Option<Line<'static>> {
    if let Some(up_next) = state.peek_queue() {
        let up_next_str = up_next.get_title().to_string();
        let total = state.playback.queue.len();

        Some(
            Line::from_iter([
                Span::from(up_next_str).fg(GOLD_FADED),
                format!(" [{total}]").fg(theme.text_faded),
            ])
            .right_aligned(),
        )
    } else {
        None
    }
}
