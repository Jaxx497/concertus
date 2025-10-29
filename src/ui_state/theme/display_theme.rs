use crate::ui_state::{Pane, UiState, theme::theme_utils::dim_color};
use ratatui::{
    style::Color,
    widgets::{BorderType, Borders},
};

pub struct DisplayTheme {
    pub bg: Color,
    pub bg_global: Color,
    pub bg_error: Color,

    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,
    pub text_selected: Color,

    pub accent: Color,
    pub selection: Color,

    pub border: Color,
    pub border_display: Borders,
    pub border_type: BorderType,

    pub progress_complete: Color,
    pub progress_incomplete: Color,
}

impl UiState {
    pub fn get_theme(&self, pane: &Pane) -> DisplayTheme {
        let theme = &self.theme_manager.active;

        match pane == self.get_pane() {
            true => DisplayTheme {
                bg: theme.surface_active,
                bg_global: theme.surface_global,
                bg_error: theme.surface_error,

                text_primary: theme.text_primary,
                text_secondary: theme.text_secondary,
                text_muted: theme.text_muted,
                text_selected: theme.text_selection,

                selection: theme.selection,

                accent: theme.accent,

                border: theme.border_active,
                border_display: theme.border_display,
                border_type: theme.border_type,

                progress_complete: theme.accent,
                progress_incomplete: theme.text_muted,
            },

            false => DisplayTheme {
                bg: theme.surface_inactive,
                bg_global: theme.surface_global,
                bg_error: theme.surface_error,

                text_primary: theme.text_muted,
                text_secondary: theme.text_secondary_in,
                text_muted: dim_color(theme.text_muted, 0.6),
                text_selected: theme.text_selection,

                selection: theme.selection_inactive,
                accent: theme.accent_inactive,

                border: theme.border_inactive,
                border_display: theme.border_display,
                border_type: theme.border_type,

                progress_complete: theme.accent,
                progress_incomplete: theme.text_muted,
            },
        }
    }
}
