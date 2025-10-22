use crate::ui_state::{Pane, UiState};
use ratatui::{
    style::Color,
    widgets::{BorderType, Borders},
};

pub struct DisplayTheme {
    pub bg: Color,
    pub bg_p: Color,
    pub border: Color,

    pub text_focused: Color,
    pub text_secondary: Color,
    pub text_faded: Color,
    pub text_highlight: Color,
    pub highlight: Color,

    pub border_display: Borders,
    pub border_type: BorderType,

    pub progress_complete: Color,
    pub progress_incomplete: Color,
}

impl UiState {
    pub fn get_theme(&self, pane: &Pane) -> DisplayTheme {
        let border_display = self.theme_manager.active.border_display;
        let border_type = self.theme_manager.active.border_type;

        match pane == self.get_pane() {
            true => DisplayTheme {
                bg: self.theme_manager.active.bg.0,
                bg_p: self.theme_manager.active.bg.2,
                border: self.theme_manager.active.border.0,
                text_focused: self.theme_manager.active.text.0,
                text_secondary: self.theme_manager.active.text2.0,
                text_faded: self.theme_manager.active.text.1,
                text_highlight: self.theme_manager.active.texth,
                highlight: self.theme_manager.active.highlight.0,

                border_display,
                border_type,

                progress_complete: self.theme_manager.active.progress.0,
                progress_incomplete: self.theme_manager.active.progress.1,
            },

            false => DisplayTheme {
                bg: self.theme_manager.active.bg.1,
                bg_p: self.theme_manager.active.bg.2,
                border: self.theme_manager.active.border.1,
                text_focused: self.theme_manager.active.text.1,
                text_secondary: self.theme_manager.active.text2.1,
                text_faded: self.theme_manager.active.text.1,
                text_highlight: self.theme_manager.active.texth,
                highlight: self.theme_manager.active.highlight.1,

                border_display,
                border_type,

                progress_complete: self.theme_manager.active.progress.1,
                progress_incomplete: self.theme_manager.active.progress.1,
            },
        }
    }
}
