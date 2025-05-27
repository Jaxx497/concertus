use super::widgets::Settings;
use super::{widgets::SongTable, AppLayout};
use super::{ErrorMsg, Progress, SearchBar, SideBar};
use crate::ui_state::Pane;
use crate::UiState;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    widgets::{Widget, *},
    Frame,
};

pub fn render(f: &mut Frame, state: &mut UiState) {
    let layout = AppLayout::new(f.area(), &state);

    SearchBar.render(layout.search_bar, f.buffer_mut(), state);
    SideBar.render(layout.side_bar, f.buffer_mut(), state);
    SongTable.render(layout.song_window, f.buffer_mut(), state);
    Progress.render(layout.progress_bar, f.buffer_mut(), state);

    // POPUPS AND ERRORS
    match (state.get_pane() == Pane::Popup, &state.get_error()) {
        (true, Some(_)) => {
            let error_win = centered_rect(40, 40, f.area());
            Clear.render(error_win, f.buffer_mut());
            ErrorMsg.render(error_win, f.buffer_mut(), state);
        }
        (true, None) => {
            let settings_popup = centered_rect(40, 40, f.area());
            Clear.render(settings_popup, f.buffer_mut());
            Settings.render(settings_popup, f.buffer_mut(), state);
        }
        (false, _) => (),
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

// fn get_up_next(state: &UiState) -> ratatui::text::Line<'_> {
//     ratatui::text::Line::from(match state.peek_queue() {
//         Some(s) => &s.title,
//         _ => "",
//     })
// }
