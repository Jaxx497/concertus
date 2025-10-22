mod display_theme;
mod theme_config;
mod theme_import;
mod theme_manager;
mod theme_utils;

pub use display_theme::DisplayTheme;
pub use theme_config::ThemeConfig;
pub use theme_manager::ThemeManager;

use ratatui::style::Color;
pub const DARK_WHITE: Color = Color::Rgb(210, 210, 210);
pub const MID_GRAY: Color = Color::Rgb(100, 100, 100);
pub const DARK_GRAY: Color = Color::Rgb(25, 25, 25);
pub const DARK_GRAY_FADED: Color = Color::Rgb(10, 10, 10);
pub const GOOD_RED: Color = Color::Rgb(255, 70, 70);
pub const GOOD_RED_DARK: Color = Color::Rgb(180, 30, 30);
pub const GOLD: Color = Color::Rgb(220, 220, 100);
pub const GOLD_FADED: Color = Color::Rgb(130, 130, 60);
