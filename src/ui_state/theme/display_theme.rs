use crate::ui_state::{
    Pane, ProgressGradient, UiState,
    theme::{
        color_utils::{fade_color, get_gradient_color},
        gradients::InactiveGradient,
    },
};
use ratatui::{
    style::Color,
    widgets::{BorderType, Borders},
};

pub struct DisplayTheme {
    pub dark: bool,
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

    pub progress_complete: ProgressGradient,
    pub progress_incomplete: InactiveGradient,
    pub progress_speed: f32,
}

impl UiState {
    pub fn get_theme(&self, pane: &Pane) -> DisplayTheme {
        let theme = &self.theme_manager.active;
        let is_dark = self.theme_manager.active.dark;

        match pane == self.get_pane() {
            true => DisplayTheme {
                dark: theme.dark,
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

                progress_complete: theme.progress.clone(),
                progress_incomplete: theme.progress_i.clone(),
                progress_speed: theme.progress_speed,
            },

            false => DisplayTheme {
                dark: theme.dark,
                bg: theme.surface_inactive,
                bg_global: theme.surface_global,
                bg_error: theme.surface_error,

                text_primary: theme.text_muted,
                text_secondary: theme.text_secondary_in,
                text_muted: fade_color(is_dark, theme.text_muted, 0.4),
                text_selected: theme.text_selection,

                selection: theme.selection_inactive,
                accent: theme.accent_inactive,

                border: theme.border_inactive,
                border_display: theme.border_display,
                border_type: theme.border_type,

                progress_complete: theme.progress.clone(),
                progress_incomplete: theme.progress_i.clone(),
                progress_speed: theme.progress_speed,
            },
        }
    }
}

impl DisplayTheme {
    pub fn get_focused_color(&self, position: f32, time: f32) -> Color {
        match &self.progress_complete {
            ProgressGradient::Static(c) => *c,
            ProgressGradient::Gradient(g) => {
                get_gradient_color(&g, position, time * self.progress_speed)
            }
        }
    }

    pub fn get_inactive_color(&self, position: f32, time: f32, amp: f32) -> Color {
        let brightness = match &self.progress_complete {
            ProgressGradient::Static(_) => 0.4,
            ProgressGradient::Gradient(g) if g.len() == 1 => 0.4,
            _ => 0.1 + (amp * 0.5),
        };

        match &self.progress_incomplete {
            InactiveGradient::Static(c) => *c,
            InactiveGradient::Gradient(g) => {
                get_gradient_color(g, position, time * self.progress_speed)
            }
            InactiveGradient::Dimmed => {
                let now_color = self.get_focused_color(position, time);
                fade_color(self.dark, now_color, brightness)
            }
            InactiveGradient::Still => {
                let now_color = self.get_focused_color(position, 0.0); // 0 to prevent movement
                fade_color(self.dark, now_color, brightness)
            }
        }
    }
}
