use super::{widgets::SongTable, AppLayout, Progress, SearchBar, SideBar};
use crate::{
    tui::{
        render_bg,
        widgets::{BufferLine, PopupManager},
    },
    ui_state::Mode,
    UiState,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::StatefulWidget,
    Frame,
};

pub fn render(f: &mut Frame, state: &mut UiState) {
    if matches!(state.get_mode(), Mode::Fullscreen) {
        let [progress, bufferline] = get_fullscreen_layout(f.area());

        Progress.render(progress, f.buffer_mut(), state);
        BufferLine.render(bufferline, f.buffer_mut(), state);

        return;
    }

    let layout = AppLayout::new(f.area(), state);
    render_bg(state, f);

    SearchBar.render(layout.search_bar, f.buffer_mut(), state);
    SideBar.render(layout.sidebar, f.buffer_mut(), state);
    SongTable.render(layout.song_window, f.buffer_mut(), state);
    Progress.render(layout.progress_bar, f.buffer_mut(), state);
    BufferLine.render(layout.buffer_line, f.buffer_mut(), state);

    if state.popup.is_open() {
        PopupManager.render(f.area(), f.buffer_mut(), state);
    }
}

fn get_fullscreen_layout(area: Rect) -> [Rect; 2] {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(99), Constraint::Length(1)])
        .areas::<2>(area)
}
