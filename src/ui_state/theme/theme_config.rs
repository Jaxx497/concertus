use crate::{
    CONFIG_DIRECTORY, THEME_DIRECTORY,
    ui_state::theme::{
        self,
        theme_import::ThemeImport,
        theme_utils::{parse_border_type, parse_borders, parse_color},
    },
};
use anyhow::Result;
use ratatui::{
    style::Color,
    widgets::{BorderType, Borders},
};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

pub struct ThemeConfig {
    pub name: String,

    pub bg_global: Color,
    pub bg_focused: Color,
    pub bg_unfocused: Color,

    pub text_focused: Color,
    pub text_secondary: Color,
    pub text_secondary_u: Color,
    pub text_unfocused: Color,
    pub text_highlighted: Color,
    pub text_highlighted_u: Color,

    pub border_focused: Color,
    pub border_unfocused: Color,

    pub progress_complete: Color,
    pub progress_incomplete: Color,

    pub border_display: Borders,
    pub border_type: BorderType,
}

impl ThemeConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file_str = std::fs::read_to_string(&path.as_ref())?;
        let config = toml::from_str::<ThemeImport>(&file_str)?;
        Self::try_from(&config)
    }

    pub fn load_or_default() -> ThemeConfig {
        let theme_dir =
            dirs::config_dir().map(|dir| dir.join(CONFIG_DIRECTORY).join(THEME_DIRECTORY));

        if let Some(ref theme_path) = theme_dir {
            if let Err(_) = fs::create_dir_all(theme_path) {
                todo!()
            }

            if let Ok(entries) = theme_path.read_dir() {
                for entry in entries.flatten() {
                    let path = entry.path();
                    println!("{:?}", path.display());

                    if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                        if let Ok(theme) = Self::load_from_file(&path) {
                            return theme;
                        }
                    }
                }
            }
        }
        Self::set_generic_theme()
    }

    pub fn set_generic_theme() -> ThemeConfig {
        use super::*;

        ThemeConfig {
            name: String::from("Concertus_Alpha"),

            bg_global: DARK_GRAY_FADED,
            bg_focused: DARK_GRAY,
            bg_unfocused: DARK_GRAY_FADED,

            text_focused: DARK_WHITE,
            text_unfocused: MID_GRAY,
            text_secondary: GOOD_RED,
            text_secondary_u: GOOD_RED_DARK,
            text_highlighted: GOLD,
            text_highlighted_u: GOLD_FADED,

            border_focused: GOLD,
            // border_unfocused: Color::Rgb(50, 50, 50),
            border_unfocused: DARK_GRAY,

            border_display: Borders::ALL,
            border_type: BorderType::Thick,

            progress_complete: GOOD_RED,
            progress_incomplete: MID_GRAY,
        }
    }
}

impl TryFrom<&ThemeImport> for ThemeConfig {
    type Error = anyhow::Error;

    fn try_from(config: &ThemeImport) -> anyhow::Result<Self> {
        let colors = &config.colors;
        Ok(ThemeConfig {
            name: config.name.clone(),

            bg_global: parse_color(&colors.bg_global)?,
            bg_focused: parse_color(&colors.bg_focused)?,
            bg_unfocused: parse_color(&colors.bg_unfocused)?,

            text_focused: parse_color(&colors.text_focused)?,
            text_unfocused: parse_color(&colors.text_unfocused)?,
            text_secondary: parse_color(&colors.text_secondary)?,
            text_secondary_u: parse_color(&colors.text_secondary_u)?,
            text_highlighted: parse_color(&colors.text_highlighted)?,
            text_highlighted_u: parse_color(&colors.text_highlighted_u)?,

            border_focused: parse_color(&colors.border_focused)?,
            border_unfocused: parse_color(&colors.border_unfocused)?,

            progress_complete: parse_color(&colors.progress_complete)?,
            progress_incomplete: parse_color(&colors.progress_incomplete)?,

            border_display: parse_borders(&config.borders.border_display),
            border_type: parse_border_type(&config.borders.border_type),
        })
    }
}
