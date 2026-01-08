use crate::{
    library::SongInfo,
    truncate_at_last_space,
    tui::widgets::{PAUSE_ICON, QUEUE_ICON, SELECTED},
    ui_state::{DisplayTheme, UiState},
};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Stylize,
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, StatefulWidget, Widget},
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
        let theme = state.theme_manager.get_display_theme(true);

        Block::new().bg(theme.bg_global).render(area, buf);

        if let Some(progress) = state
            .get_library_refresh_progress()
            .filter(|p| *p > 1 && *p < 100)
        {
            let desc = state.get_library_refresh_detail().unwrap_or_default();
            let label = format!("{desc} | {progress}%");
            let guage = Gauge::default()
                .block(Block::new().borders(Borders::NONE))
                .gauge_style(theme.selection)
                .fg(theme.text_selected)
                .label(label)
                .percent(progress as u16 - 1);

            guage.render(area, buf);
            return;
        }

        let [left, center, right] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(40),
                Constraint::Percentage(30),
            ])
            .areas(area);

        let selection_count = state.get_multi_select_indices().len();

        get_multi_selection(selection_count, &theme).render(left, buf);
        playing_title(state, &theme, center.width as usize).render(center, buf);
        queue_display(state, &theme, right.width as usize).render(right, buf);
    }
}

const SEPARATOR_LEN: usize = 3;
const MIN_TITLE_LEN: usize = 20;
const MIN_ARTIST_LEN: usize = 15;

fn playing_title(state: &UiState, theme: &DisplayTheme, width: usize) -> Option<Line<'static>> {
    let song = state.get_now_playing()?;
    let decorator = &state.get_decorator();

    let separator = match state.is_paused() {
        true => Span::from(format!(" {PAUSE_ICON} "))
            .fg(theme.text_primary)
            .rapid_blink(),
        false => Span::from(format!(" {decorator} ")).fg(theme.text_muted),
    };

    let title = song.get_title().to_string();
    let artist = song.get_artist().to_string();

    let title_len = title.chars().count();
    let artist_len = artist.chars().count();

    if width >= title_len + SEPARATOR_LEN + artist_len {
        Some(
            Line::from_iter([
                Span::from(title).fg(theme.text_secondary),
                Span::from(separator),
                Span::from(artist).fg(theme.text_muted),
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
                Span::from(truncated_artist).fg(theme.text_muted),
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

fn get_multi_selection(size: usize, theme: &DisplayTheme) -> Option<Line<'static>> {
    let output = match size {
        0 => return None,
        x => format!("{x:>3} {} ", SELECTED)
            .fg(theme.accent)
            .into_left_aligned_line(),
    };

    Some(output)
}

const BAD_WIDTH: usize = 22;
fn queue_display(state: &UiState, theme: &DisplayTheme, width: usize) -> Option<Line<'static>> {
    let up_next_str = state.peek_queue()?.get_title();

    // [width - 5] should produce enough room to avoid overlapping with other displays
    let truncated = truncate_at_last_space(up_next_str, width - 5);

    let up_next_line = Span::from(truncated).fg(state.theme_manager.active.selection_inactive);

    let total = state.playback.queue_len();
    let queue_total = format!(" [{total}] ").fg(theme.text_muted);

    match width < BAD_WIDTH {
        true => Some(
            Line::from_iter([Span::from(QUEUE_ICON).fg(theme.text_muted), queue_total])
                .right_aligned(),
        ),

        false => Some(
            Line::from_iter([
                Span::from(QUEUE_ICON).fg(theme.text_muted),
                " ".into(),
                up_next_line,
                queue_total,
            ])
            .right_aligned(),
        ),
    }
}
