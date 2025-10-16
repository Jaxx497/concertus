mod theme_config;
mod theme_import;
mod theme_manager;
mod theme_utils;

use crate::ui_state::{Pane, UiState};
use ratatui::{
    style::Color,
    widgets::{BorderType, Borders},
};

pub use theme_config::ThemeConfig;
pub use theme_manager::ThemeManager;

const DARK_WHITE: Color = Color::Rgb(210, 210, 210);
const MID_GRAY: Color = Color::Rgb(100, 100, 100);
pub const DARK_GRAY: Color = Color::Rgb(25, 25, 25);
pub const DARK_GRAY_FADED: Color = Color::Rgb(10, 10, 10);
pub const GOOD_RED: Color = Color::Rgb(255, 70, 70);
pub const GOOD_RED_DARK: Color = Color::Rgb(180, 30, 30);
pub const GOLD: Color = Color::Rgb(220, 220, 100);
pub const GOLD_FADED: Color = Color::Rgb(130, 130, 60);

pub struct DisplayTheme {
    pub bg: Color,
    pub bg_panel: Color,
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
                bg: self.theme_manager.active.bg_global,

                bg_panel: self.theme_manager.active.bg_focused,
                border: self.theme_manager.active.border_focused,
                text_focused: self.theme_manager.active.text_focused,
                text_secondary: self.theme_manager.active.text_secondary,
                text_faded: self.theme_manager.active.text_unfocused,
                text_highlight: self.theme_manager.active.text_highlight,
                highlight: self.theme_manager.active.highlight,

                border_display,
                border_type,

                progress_complete: self.theme_manager.active.progress_complete,
                progress_incomplete: self.theme_manager.active.progress_incomplete,
            },

            false => DisplayTheme {
                bg: self.theme_manager.active.bg_global,

                bg_panel: self.theme_manager.active.bg_unfocused,
                border: self.theme_manager.active.border_unfocused,
                text_focused: self.theme_manager.active.text_unfocused,
                text_secondary: self.theme_manager.active.text_secondary_u,
                text_faded: self.theme_manager.active.text_unfocused,
                text_highlight: self.theme_manager.active.text_highlight,
                highlight: self.theme_manager.active.highlight_u,

                border_display,
                border_type,

                progress_complete: self.theme_manager.active.progress_complete,
                progress_incomplete: self.theme_manager.active.progress_incomplete,
            },
        }
    }
}
