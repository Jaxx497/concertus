use crate::ui_state::theme::{
    theme_import::ThemeImport,
    theme_utils::{parse_border_type, parse_borders, parse_color},
};
use anyhow::Result;
use ratatui::{
    style::Color,
    widgets::{BorderType, Borders},
};
use std::path::Path;

#[derive(Clone)]
pub struct ThemeConfig {
    pub name: String,

    pub bg: (Color, Color, Color),
    pub text: (Color, Color),
    pub text2: (Color, Color),
    pub texth: Color,
    pub highlight: (Color, Color),
    pub border: (Color, Color),
    pub progress: (Color, Color),

    pub border_display: Borders,
    pub border_type: BorderType,
}

impl ThemeConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file_str = std::fs::read_to_string(&path.as_ref())?;
        let config = toml::from_str::<ThemeImport>(&file_str)?;
        Self::try_from(&config)
    }

    pub fn set_generic_theme() -> ThemeConfig {
        use super::*;

        ThemeConfig {
            name: String::from("Concertus_Alpha"),

            bg: (DARK_GRAY, DARK_GRAY_FADED, DARK_GRAY_FADED),
            text: (DARK_WHITE, MID_GRAY),
            text2: (GOOD_RED, GOOD_RED_DARK),
            texth: DARK_GRAY,
            highlight: (GOLD, GOLD_FADED),
            border: (GOLD, DARK_GRAY),
            progress: (GOOD_RED, MID_GRAY),

            border_display: Borders::ALL,
            border_type: BorderType::Thick,
        }
    }
}

impl TryFrom<&ThemeImport> for ThemeConfig {
    type Error = anyhow::Error;

    fn try_from(config: &ThemeImport) -> anyhow::Result<Self> {
        let colors = &config.colors;

        let bg_focused = parse_color(&colors.bg_focused)?;
        let bg_unfocused = parse_color(&colors.bg_unfocused)?;
        let bg_global = parse_color(&colors.bg_progress)?;

        let text_focused = parse_color(&colors.text_focused)?;
        let text_unfocused = parse_color(&colors.text_unfocused)?;

        let text_secondary = parse_color(&colors.text_secondary)?;
        let text_secondary_u = parse_color(&colors.text_secondary_u)?;
        let text_highlight = parse_color(&colors.text_highlight)?;

        let highlight = parse_color(&colors.highlight)?;
        let highlight_u = parse_color(&colors.highlight_u)?;

        let border_focused = parse_color(&colors.border_focused)?;
        let border_unfocused = parse_color(&colors.border_unfocused)?;

        let progress_complete = parse_color(&colors.progress_complete)?;
        let progress_incomplete = parse_color(&colors.progress_incomplete)?;

        Ok(ThemeConfig {
            name: config.name.clone(),

            bg: (bg_focused, bg_unfocused, bg_global),
            text: (text_focused, text_unfocused),
            text2: (text_secondary, text_secondary_u),
            texth: text_highlight,

            highlight: (highlight, highlight_u),
            border: (border_focused, border_unfocused),

            progress: (progress_complete, progress_incomplete),

            border_display: parse_borders(&config.borders.border_display),
            border_type: parse_border_type(&config.borders.border_type),
        })
    }
}
