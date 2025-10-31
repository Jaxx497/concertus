use crate::ui_state::{
    Pane, ProgressGradient, UiState, dim_color,
    theme::{color_utils::get_gradient_color, gradients::InactiveGradient},
};
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

    pub progress_complete: ProgressGradient,
    pub progress_incomplete: InactiveGradient,
    pub oscillo: ProgressGradient,
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

                progress_complete: theme.progress.clone(),
                progress_incomplete: theme.progress_i.clone(),
                oscillo: theme.oscillo.clone(),
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

                progress_complete: theme.progress.clone(),
                progress_incomplete: theme.progress_i.clone(),
                oscillo: theme.oscillo.clone(),
            },
        }
    }
}

impl DisplayTheme {
    pub fn get_focused_color(&self, position: f32, time: f32) -> Color {
        match &self.progress_complete {
            ProgressGradient::Static(c) => *c,
            ProgressGradient::Gradient(g) => get_gradient_color(&g, position, time),
        }
    }

    pub fn get_oscilloscope_color(&self, position: f32, time: f32) -> Color {
        match &self.oscillo {
            ProgressGradient::Static(c) => *c,
            ProgressGradient::Gradient(g) => get_gradient_color(&g, position, time),
        }
    }

    pub fn get_inactive_color(&self, position: f32, time: f32, amp: f32) -> Color {
        let brightness = match &self.progress_complete {
            ProgressGradient::Static(_) => 0.5,
            ProgressGradient::Gradient(x) if x.len() == 1 => 0.5,
            _ => 0.2 + (amp * 0.5),
        };

        match &self.progress_incomplete {
            InactiveGradient::Static(c) => *c,
            InactiveGradient::Gradient(g) => get_gradient_color(g, position, time),
            InactiveGradient::Dimmed => {
                let now_color = self.get_focused_color(position, time);
                dim_color(now_color, brightness)
            }
            InactiveGradient::Still => {
                let now_color = self.get_focused_color(position, 0.0); // 0 to prevent movement
                dim_color(now_color, brightness)
            }
        }
    }
}
