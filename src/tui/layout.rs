use crate::ui_state::{Mode, ProgressDisplay, UiState};
use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct AppLayout {
    pub sidebar: Rect,
    pub search_bar: Rect,
    pub song_window: Rect,
    pub progress_bar: Rect,
    pub buffer_line: Rect,
}

impl AppLayout {
    pub fn new(area: Rect, state: &UiState) -> Self {
        let wf_height = if state.display_waveform() {
            match (state.which_display_style(), area.height > 25) {
                (ProgressDisplay::Waveform, true) => 6,
                _ => 4,
            }
        } else {
            0
        };

        let search_height = match state.get_mode() == Mode::Search {
            true => 5,
            false => 0,
        };

        let buffer_line_height = match !state.is_not_playing() || !state.bulk_select_empty() {
            true => 1,
            false => 0,
        };

        let [upper_block, progress_bar, buffer_line] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(16),
                Constraint::Length(wf_height),
                Constraint::Length(buffer_line_height),
            ])
            .areas(area);

        let [sidebar, _, upper_block] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(state.display_state.sidebar_percent),
                Constraint::Length(1),
                Constraint::Fill(1),
            ])
            .areas(upper_block);

        let [search_bar, song_window] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(search_height), Constraint::Fill(100)])
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
