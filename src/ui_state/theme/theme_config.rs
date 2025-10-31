use crate::ui_state::{
    ProgressGradient,
    theme::{
        GOOD_RED_DARK,
        gradients::InactiveGradient,
        theme_import::ThemeImport,
        theme_utils::{parse_border_type, parse_borders, parse_color},
    },
};
use anyhow::{Result, anyhow};
use ratatui::{
    style::Color,
    widgets::{BorderType, Borders},
};
use std::path::Path;

#[derive(Clone)]
pub struct ThemeConfig {
    pub name: String,

    // Surface Colors
    pub surface_global: Color,   // Global bg
    pub surface_active: Color,   // Focused pane
    pub surface_inactive: Color, // Inactive pane
    pub surface_error: Color,    // Error popup bg

    // Text colors
    pub text_primary: Color,      // Focused text
    pub text_secondary: Color,    // Accented text
    pub text_secondary_in: Color, // Accented text
    pub text_muted: Color,        // Inactive/quiet text
    pub text_selection: Color,    // Text inside of selection bar

    // Border colors
    pub border_active: Color,   // Border highlight
    pub border_inactive: Color, // Border Inactive

    // Selection colors
    pub selection: Color,          // Selection Bar color
    pub selection_inactive: Color, // Selection inactive

    // Accent
    pub accent: Color,
    pub accent_inactive: Color,

    // Border configuration
    pub border_display: Borders,
    pub border_type: BorderType,

    pub progress: ProgressGradient,
    pub progress_i: InactiveGradient,

    pub oscillo: ProgressGradient,
}

impl ThemeConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file_str = std::fs::read_to_string(&path.as_ref())?;
        let config = toml::from_str::<ThemeImport>(&file_str)?;
        let mut theme = Self::try_from(&config)?;

        theme.name = path
            .as_ref()
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or(anyhow!("Could not identify theme name"))?
            .to_string();

        Ok(theme)
    }
}

impl TryFrom<&ThemeImport> for ThemeConfig {
    type Error = anyhow::Error;

    fn try_from(config: &ThemeImport) -> anyhow::Result<Self> {
        let colors = &config.colors;

        let surface_global = parse_color(&colors.surface_global)?;
        let surface_active = parse_color(&colors.surface_active).unwrap_or(surface_global); // Fallback to surface_global
        let surface_inactive = parse_color(&colors.surface_inactive).unwrap_or(surface_global); // Fallback to surface_global
        let surface_error = parse_color(&colors.surface_error).unwrap_or(GOOD_RED_DARK);

        let text_primary = parse_color(&colors.text_primary)?;
        let text_secondary = parse_color(&colors.text_secondary)?;
        let text_secondary_in = parse_color(&colors.text_secondary_in)?;
        let text_selection = parse_color(&colors.text_selection).unwrap_or(surface_global);
        let text_muted = parse_color(&colors.text_muted)?;

        let border_active = parse_color(&colors.border_active)?;
        let border_inactive = parse_color(&colors.border_inactive).unwrap_or(border_active);

        let accent = parse_color(&colors.accent)?;
        let accent_inactive = parse_color(&colors.accent_inactive).unwrap_or(accent);

        let selection = parse_color(&colors.selection).unwrap_or(border_active);
        let selection_inactive = parse_color(&colors.selection_inactive).unwrap_or(selection);

        let progress = ProgressGradient::from_raw(&colors.waveform)?;
        let progress_i = InactiveGradient::from_raw(&colors.waveform_i)?;
        let oscillo = ProgressGradient::from_raw(&colors.oscilloscope)?;

        Ok(ThemeConfig {
            name: String::new(),

            surface_global,
            surface_active,
            surface_inactive,
            surface_error,

            text_primary,
            text_secondary,
            text_secondary_in,
            text_muted,
            text_selection,

            border_active,
            border_inactive,

            selection,
            selection_inactive,

            accent,
            accent_inactive,

            border_display: parse_borders(&config.borders.border_display),
            border_type: parse_border_type(&config.borders.border_type),

            progress,
            progress_i,
            oscillo,
        })
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        use super::*;

        ThemeConfig {
            name: String::from("Concertus_Alpha"),

            surface_global: DARK_GRAY_FADED,
            surface_active: DARK_GRAY,
            surface_inactive: DARK_GRAY_FADED,
            surface_error: GOOD_RED_DARK,

            text_primary: DARK_WHITE,
            text_muted: MID_GRAY,
            text_selection: DARK_GRAY,
            text_secondary: GOOD_RED,
            text_secondary_in: GOOD_RED_DARK,

            border_active: GOLD,
            border_inactive: DARK_GRAY_FADED,

            selection: GOLD,
            selection_inactive: GOLD_FADED,

            accent: GOLD,
            accent_inactive: GOLD_FADED,

            border_display: Borders::ALL,
            border_type: BorderType::Rounded,

            progress: ProgressGradient::Gradient(Vec::from([DARK_WHITE, GOOD_RED_DARK, DARK_GRAY])),
            progress_i: InactiveGradient::Dimmed,
            oscillo: ProgressGradient::Gradient(Vec::from([DARK_WHITE, GOLD, DARK_GRAY])),
        }
    }
}
