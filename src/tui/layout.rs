use crate::ui_state::{Mode, UiState};
use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct AppLayout {
    pub side_bar: Rect,
    pub search_bar: Rect,
    pub song_window: Rect,
    pub progress_bar: Rect,
}

impl AppLayout {
    pub fn new(area: Rect, state: &UiState) -> Self {
        let (wf_splitter, wf_height) = match state.get_now_playing().is_some() {
            true => (1, 6),
            false => (0, 0),
        };

        let (search_splitter, search_height) = match state.get_mode() == Mode::Search {
            true => (1, 3),
            false => (0, 0),
        };

        let [upper_block, _, progress_bar] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(12),
                Constraint::Length(wf_splitter),
                Constraint::Length(wf_height),
            ])
            .areas(area);

        let [side_bar, _, upper_block] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(40),
                Constraint::Length(2),
                Constraint::Min(40),
            ])
            .areas(upper_block);

        let [search_bar, _, song_window] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(search_height),
                Constraint::Length(search_splitter),
                Constraint::Fill(100),
            ])
            .areas(upper_block);

        AppLayout {
            side_bar,
            search_bar,
            song_window,
            progress_bar,
        }
    }
}
