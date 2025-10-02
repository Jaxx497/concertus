use super::{
    AppLayout, ErrorMsg, Progress, SearchBar, SideBar,
    widgets::{Settings, SongTable},
};
use crate::{
    UiState,
    tui::widgets::{BufferLine, PlaylistPopup},
    ui_state::{Mode, PopupType},
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Stylize,
    widgets::{Widget, *},
};

pub fn render(f: &mut Frame, state: &mut UiState) {
    if matches!(state.get_mode(), Mode::Fullscreen) {
        let [progress, bufferline] = get_full_screen_layout(f.area());

        Progress.render(progress, f.buffer_mut(), state);
        BufferLine.render(bufferline, f.buffer_mut(), state);

        return;
    }

    let layout = AppLayout::new(f.area(), state);

    Block::new()
        .bg(state.theme.bg_global)
        .render(f.area(), f.buffer_mut());

    SearchBar.render(layout.search_bar, f.buffer_mut(), state);
    SideBar.render(layout.sidebar, f.buffer_mut(), state);
    SongTable.render(layout.song_window, f.buffer_mut(), state);
    Progress.render(layout.progress_bar, f.buffer_mut(), state);
    BufferLine.render(layout.buffer_line, f.buffer_mut(), state);

    if state.popup.is_open() {
        let popup_rect = match &state.popup.current {
            PopupType::Playlist(_) => centered_rect(35, 40, f.area()),
            PopupType::Settings(_) => centered_rect(35, 35, f.area()),
            PopupType::Error(_) => centered_rect(40, 30, f.area()),
            _ => centered_rect(30, 30, f.area()),
        };

        Clear.render(popup_rect, f.buffer_mut());
        match &state.popup.current {
            PopupType::Playlist(_) => PlaylistPopup.render(popup_rect, f.buffer_mut(), state),
            PopupType::Settings(_) => Settings.render(popup_rect, f.buffer_mut(), state),
            PopupType::Error(_) => ErrorMsg.render(popup_rect, f.buffer_mut(), state),
            _ => (),
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}

fn get_full_screen_layout(area: Rect) -> [Rect; 2] {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(99), Constraint::Length(1)])
        .areas::<2>(area)
}
