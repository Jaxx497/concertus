use crate::{
    domain::SongInfo,
    truncate_at_last_space,
    tui::widgets::{PAUSE_ICON, QUEUE_ICON, SELECTED},
    ui_state::{DisplayTheme, UiState},
};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Stylize,
    text::{Line, Span},
    widgets::{Block, StatefulWidget, Widget},
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

        Block::new().bg(theme.bg_p).render(area, buf);

        let [left, center, right] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(40),
                Constraint::Percentage(30),
            ])
            .areas(area);

        let selection_count = state.get_bulk_select().len();

        get_bulk_selection(selection_count, &theme).render(left, buf);
        playing_title(state, &theme, center.width as usize).render(center, buf);
        queue_display(state, &theme, right.width as usize).render(right, buf);
    }
}

const SEPARATOR_LEN: usize = 3;
const MIN_TITLE_LEN: usize = 20;
const MIN_ARTIST_LEN: usize = 15;

fn playing_title(state: &UiState, theme: &DisplayTheme, width: usize) -> Option<Line<'static>> {
    let song = state.get_now_playing()?;

    let separator = match state.is_paused() {
        true => Span::from(format!(" {PAUSE_ICON} "))
            .fg(theme.text_focused)
            .rapid_blink(),
        false => Span::from(" ✧ ").fg(theme.text_faded),
    };

    let title = song.get_title().to_string();
    let artist = song.get_artist().to_string();

    let title_len = title.chars().count();
    let artist_len = artist.chars().count();

    if width >= title_len + SEPARATOR_LEN + artist_len {
        Some(
            Line::from_iter([
                Span::from(title).fg(theme.text_secondary),
                Span::from(separator).fg(theme.text_focused),
                Span::from(artist).fg(theme.text_faded),
            ])
            .centered(),
        )
    } else if width >= MIN_TITLE_LEN + SEPARATOR_LEN + MIN_ARTIST_LEN {
        let available_space = width.saturating_sub(SEPARATOR_LEN);
        let title_space = (available_space * 2) / 3;
        let artist_space = available_space.saturating_sub(title_space);

        let truncated_title = truncate_at_last_space(&title, title_space);
        let truncated_artist = truncate_at_last_space(&artist, artist_space);

        Some(
            Line::from_iter([
                Span::from(truncated_title).fg(theme.text_secondary),
                separator,
                Span::from(truncated_artist).fg(theme.text_faded),
            ])
            .centered(),
        )
    } else {
        match state.is_paused() {
            true => {
                let truncated_title = truncate_at_last_space(&title, title_len - SEPARATOR_LEN);
                Some(
                    Line::from_iter([
                        separator,
                        Span::from(truncated_title).fg(theme.text_secondary),
                    ])
                    .centered(),
                )
            }
            false => {
                let truncated_title = truncate_at_last_space(&title, width);
                Some(Line::from(Span::from(truncated_title).fg(theme.text_secondary)).centered())
            }
        }
    }
}

fn get_bulk_selection(size: usize, theme: &DisplayTheme) -> Option<Line<'static>> {
    let output = match size {
        0 => return None,
        x => format!("{x:>3} {} ", SELECTED)
            .fg(theme.highlight)
            .into_left_aligned_line(),
    };

    Some(output)
}

const BAD_WIDTH: usize = 22;
fn queue_display(state: &UiState, theme: &DisplayTheme, width: usize) -> Option<Line<'static>> {
    let up_next = state.peek_queue()?;

    let alert = state
        .get_now_playing()
        .map(|np| {
            let duration = np.duration.as_secs_f32();
            let elapsed = state.get_playback_elapsed().as_secs_f32();

            // Flash when less than 3 seconds left on now_playing
            (duration - elapsed) < 3.0
        })
        .unwrap_or(false);

    let up_next_str = up_next.get_title();

    // [width - 5] should produce enough room to avoid overlapping with other displays
    let truncated = truncate_at_last_space(up_next_str, width - 5);

    let up_next_line = match alert {
        true => Span::from(truncated)
            .fg(state.theme_manager.active.highlight.1)
            .rapid_blink(),
        false => Span::from(truncated).fg(state.theme_manager.active.highlight.1),
    };

    let total = state.playback.queue.len();
    let queue_total = format!(" [{total}] ").fg(theme.text_faded);

    match width < BAD_WIDTH {
        true => Some(
            Line::from_iter([Span::from(QUEUE_ICON).fg(theme.text_faded), queue_total])
                .right_aligned(),
        ),

        false => Some(
            Line::from_iter([
                Span::from(QUEUE_ICON).fg(theme.text_faded),
                " ".into(),
                up_next_line,
                queue_total,
            ])
            .right_aligned(),
        ),
    }
}
