use ratatui::{
    layout::{Constraint, Layout, Rect},
    widgets::{Clear, StatefulWidget, Widget},
};

use crate::{
    tui::{
        ErrorMsg,
        widgets::{PlaylistPopup, RootManager, ThemeManager},
    },
    ui_state::{PopupType, UiState},
};

pub struct PopupManager;
impl StatefulWidget for PopupManager {
    type State = UiState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let popup_rect = match &state.popup.current {
            PopupType::Playlist(_) => centered_rect(35, 40, area),
            PopupType::Settings(_) => centered_rect(40, 40, area),
            PopupType::ThemeManager => centered_rect(40, 40, area),
            PopupType::Error(_) => centered_rect(40, 35, area),
            _ => return,
        };

        Clear.render(popup_rect, buf);
        match &state.popup.current {
            PopupType::Playlist(_) => PlaylistPopup.render(popup_rect, buf, state),
            PopupType::Settings(_) => RootManager.render(popup_rect, buf, state),

            PopupType::ThemeManager => ThemeManager.render(popup_rect, buf, state),
            PopupType::Error(_) => ErrorMsg.render(popup_rect, buf, state),
            _ => unreachable!(),
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
