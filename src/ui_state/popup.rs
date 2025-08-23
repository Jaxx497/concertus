use ratatui::widgets::ListState;
use tui_textarea::TextArea;

use crate::ui_state::{new_textarea, playlist::PlaylistAction, Pane, SettingsMode, UiState};

#[derive(PartialEq)]
pub enum PopupType {
    None,
    Error(String),
    Settings(SettingsMode),
    Playlist(PlaylistAction),
}

pub struct PopupState {
    pub current: PopupType,
    pub input: TextArea<'static>,
    pub selection: ListState,
    pub cached: Pane,
}

impl PopupState {
    pub(crate) fn new() -> PopupState {
        PopupState {
            current: PopupType::None,
            input: new_textarea(""),
            selection: ListState::default(),
            cached: Pane::Popup,
        }
    }

    fn open(&mut self, popup: PopupType) {
        match &popup {
            PopupType::Playlist(PlaylistAction::Create) => {
                self.input.set_placeholder_text(" Enter playlist name: ");
                self.input.select_all();
                self.input.cut();
            }
            PopupType::Settings(SettingsMode::ViewRoots) => {
                self.input.select_all();
                self.input.cut();
            }
            PopupType::Settings(SettingsMode::AddRoot) => {
                self.input
                    .set_placeholder_text(" Enter path to directory: ");
                self.input.select_all();
                self.input.cut();
            }

            _ => (),
        }
        self.current = popup
    }

    pub fn is_open(&self) -> bool {
        self.current != PopupType::None
    }

    fn close(&mut self) -> Pane {
        self.current = PopupType::None;
        self.input.select_all();
        self.input.cut();

        self.cached.clone()
    }

    fn set_cached_pane(&mut self, pane: Pane) {
        self.cached = pane
    }
}

impl UiState {
    pub fn show_popup(&mut self, popup: PopupType) {
        self.popup.open(popup);
        if self.popup.cached == Pane::Popup {
            let current_pane = self.get_pane().clone();
            self.popup.set_cached_pane(current_pane);
            self.set_pane(Pane::Popup);
        }
    }

    pub fn close_popup(&mut self) {
        let pane = self.popup.close();
        self.popup.cached = Pane::Popup;
        self.set_pane(pane);
    }
}
