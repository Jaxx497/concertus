use crate::ui_state::{Mode, ProgressDisplay, UiState};
use ratatui::layout::{Constraint, Layout, Rect};

pub struct AppLayout {
    pub sidebar: Rect,
    pub search_bar: Rect,
    pub song_window: Rect,
    pub progress_bar: Rect,
    pub buffer_line: Rect,
}

impl AppLayout {
    pub fn new(area: Rect, state: &mut UiState) -> Self {
        let prog_height = match state.display_progress() {
            true => match (state.get_progress_display(), area.height > 20) {
                (ProgressDisplay::Waveform | ProgressDisplay::Oscilloscope, true) => 6,
                _ => 3,
            },
            false => 0,
        };

        let search_height = match state.get_mode() == Mode::Search {
            true => 5,
            false => 0,
        };

        let buffer_line_height = match state.player_is_active()
            || !state.multi_select_empty()
            || state.get_library_refresh_progress().is_some()
        {
            true => 1,
            false => 0,
        };

        let [upper_block, progress_bar, buffer_line] = Layout::vertical([
            Constraint::Min(16),
            Constraint::Length(prog_height),
            Constraint::Length(buffer_line_height),
        ])
        .areas(area);

        let [sidebar, _, upper_block] = Layout::horizontal([
            Constraint::Percentage(state.display_state.sidebar_percent),
            Constraint::Length(0),
            Constraint::Fill(1),
        ])
        .areas(upper_block);

        let [search_bar, song_window] =
            Layout::vertical([Constraint::Length(search_height), Constraint::Fill(100)])
                .areas(upper_block);

        AppLayout {
            sidebar,
            search_bar,
            song_window,
            progress_bar,
            buffer_line,
        }
    }
}
