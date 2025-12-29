mod color_utils;
mod display_theme;
mod gradients;
mod import;
mod theme_config;
mod theme_manager;
mod theme_utils;

pub use color_utils::{SHARP_FACTOR, fade_color, get_gradient_color};
pub use display_theme::DisplayTheme;
pub use gradients::{InactiveGradient, ProgressGradient};
pub use import::{ProgressGradientRaw, ThemeImport};
pub use theme_config::ThemeConfig;
pub use theme_manager::ThemeManager;

use ratatui::style::Color;

const DARK_WHITE: Color = Color::Rgb(210, 210, 213);
const MID_GRAY: Color = Color::Rgb(100, 100, 103);
const DARK_GRAY: Color = Color::Rgb(25, 25, 28);
const DARK_GRAY_FADED: Color = Color::Rgb(15, 15, 18);
const GOOD_RED: Color = Color::Rgb(255, 70, 70);
const GOOD_RED_DARK: Color = Color::Rgb(180, 30, 30);
const GOLD: Color = Color::Rgb(220, 220, 100);
const GOLD_FADED: Color = Color::Rgb(130, 130, 60);
