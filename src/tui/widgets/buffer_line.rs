use crate::{
    domain::SongInfo,
    truncate_at_last_space,
    tui::widgets::{PAUSE_ICON, SELECTED},
    ui_state::{DisplayTheme, GOLD_FADED, UiState},
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

        let [left, center, right] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(40),
                Constraint::Percentage(30),
            ])
            .areas(area);

        let selection_count = state.get_bulk_sel().len();

        get_bulk_selection(selection_count).render(left, buf);
        playing_title(state, &theme, center.width as usize).render(center, buf);
        queue_display(state, &theme, right.width as usize).render(right, buf);
    }
}

fn playing_title(state: &UiState, theme: &DisplayTheme, width: usize) -> Option<Line<'static>> {
    let separator = match state.is_paused() {
        true => Span::from(format!(" {PAUSE_ICON} "))
            .fg(theme.text_focused)
            .rapid_blink(),
        false => Span::from(" ✧ ").fg(theme.text_faded),
    };

    if let Some(s) = state.get_now_playing() {
        let available_width = width.saturating_sub(3); // 3 is the length of " ✧ "

        let title = s.get_title();
        let artist = s.get_artist();

        let (final_title, final_artist) =
            if title.chars().count() + artist.chars().count() <= available_width {
                (title.to_string(), artist.to_string())
            } else if title.chars().count() <= available_width * 2 / 3 {
                // Title fits in 2/3, truncate artist
                let artist_space = available_width.saturating_sub(title.chars().count());
                (
                    title.to_string(),
                    truncate_at_last_space(artist, artist_space),
                )
            } else {
                let title_space = (available_width * 3) / 5;
                let artist_space = available_width.saturating_sub(title_space);
                (
                    truncate_at_last_space(title, available_width),
                    truncate_at_last_space(artist, artist_space),
                )
            };

        Some(
            Line::from_iter([
                Span::from(final_title).fg(theme.text_secondary),
                Span::from(separator).fg(theme.text_focused),
                Span::from(final_artist).fg(theme.text_faded),
            ])
            .centered(),
        )
    } else {
        None
    }
}

fn get_bulk_selection(size: usize) -> Option<Line<'static>> {
    let output = match size {
        0 => return None,
        x => format!("{x:>3} {} ", SELECTED)
            .fg(GOLD_FADED)
            .into_left_aligned_line(),
    };

    Some(output)
}

fn queue_display(state: &UiState, theme: &DisplayTheme, width: usize) -> Option<Line<'static>> {
    let up_next = state.peek_queue()?;

    let alert = state
        .get_now_playing()
        .map(|np| {
            let duration = np.duration.as_secs_f32();
            let elapsed = state.get_playback_elapsed().as_secs_f32();

            (duration - elapsed) < 3.0
        })
        .unwrap_or(false);

    let up_next_str = up_next.get_title();
    let truncated = truncate_at_last_space(up_next_str, width - 12);
    let total = state.playback.queue.len();

    let output = Line::from_iter([
        Span::from("Up next ✧ ").fg(theme.text_faded),
        Span::from(truncated).fg(GOLD_FADED),
        format!(" [{total}] ").fg(theme.text_faded),
    ])
    .right_aligned();

    Some(if alert { output.rapid_blink() } else { output })
}
