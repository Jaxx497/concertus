use super::widgets::Settings;
use super::{widgets::SongTable, AppLayout};
use super::{ErrorMsg, Progress, SearchBar, SideBar};
use crate::tui::widgets::{BufferLine, PlaylistPopup};
use crate::ui_state::PopupType;
use crate::UiState;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    widgets::{Widget, *},
    Frame,
};

pub fn render(f: &mut Frame, state: &mut UiState) {
    let layout = AppLayout::new(f.area(), &state);

    SearchBar.render(layout.search_bar, f.buffer_mut(), state);
    SideBar.render(layout.sidebar, f.buffer_mut(), state);
    SongTable.render(layout.song_window, f.buffer_mut(), state);
    Progress.render(layout.progress_bar, f.buffer_mut(), state);
    BufferLine.render(layout.buffer_line, f.buffer_mut(), state);

    if state.popup.is_open() {
        let popup_rect = match &state.popup.current {
            PopupType::Playlist(_) => centered_rect(30, 40, f.area()),
            PopupType::Settings(_) => centered_rect(30, 40, f.area()),
            PopupType::Error(_) => centered_rect(40, 35, f.area()),
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
