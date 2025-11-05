use crate::ui_state::{
    ProgressGradient, UiState,
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
    pub fn get_decorator(&self) -> &str {
        &self.theme_manager.active.decorator
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
