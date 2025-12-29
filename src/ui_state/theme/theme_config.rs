use crate::ui_state::{
    ProgressGradient, ThemeImport,
    theme::{
        gradients::InactiveGradient,
        theme_utils::{parse_borders, parse_display},
    },
};
use anyhow::{Result, anyhow};
use ratatui::{
    style::Color,
    symbols::Marker,
    widgets::{BorderType, Borders},
};
use std::{path::Path, rc::Rc, sync::Arc};

#[derive(Clone)]
pub struct ThemeConfig {
    pub name: String,
    pub is_dark: bool,

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

    // Progress Displays
    pub progress_played: ProgressGradient,
    pub progress_unplayed: InactiveGradient,
    pub progress_speed: f32,
    pub bar_active: String,
    pub bar_inactive: String,
    pub waveform_style: Marker,
    pub oscilloscope_style: Marker,

    pub decorator: Rc<String>,
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
        let progress = &config.progress;

        let surface_global = *colors.surface_global;
        let surface_active = *colors.surface_active;
        let surface_inactive = *colors.surface_inactive;
        let surface_error = *colors.surface_error;

        let text_primary = *colors.text_primary;
        let text_secondary = *colors.text_secondary;
        let text_secondary_in = *colors.text_secondary_in;
        let text_selection = *colors.text_selection;
        let text_muted = *colors.text_muted;

        let border_active = *colors.border_active;
        let border_inactive = *colors.border_inactive;

        let accent = *colors.accent;
        let accent_inactive = *colors.accent_inactive;

        let selection = *colors.selection;
        let selection_inactive = *colors.selection_inactive;

        let border_display = parse_borders(&config.borders.border_display);

        let progress_played = ProgressGradient::from_raw(&progress.elapsed)?;
        let progress_unplayed = InactiveGradient::from_raw(&progress.unplayed)?;
        let progress_speed = progress.speed / -10.0;

        let bar_active = progress.bar_elapsed.to_owned();
        let bar_inactive = progress.bar_unplayed.to_owned();

        let waveform_style = parse_display(&progress.waveform_style);
        let oscilloscope_style = parse_display(&progress.oscilloscope_style);

        let decorator = Rc::from(config.extras.decorator.to_owned());
        let is_dark = config.extras.is_dark;

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

            border_display,
            border_type: config.borders.border_type,

            progress_played,
            progress_unplayed,
            progress_speed,

            bar_active,
            bar_inactive,
            waveform_style,
            oscilloscope_style,

            decorator,
            is_dark,
        })
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        use super::*;

        ThemeConfig {
            name: String::from("Concertus_Alpha"),
            is_dark: true,

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

            progress_played: ProgressGradient::Gradient(Arc::from([
                DARK_WHITE,
                GOOD_RED_DARK,
                DARK_GRAY,
            ])),
            progress_unplayed: InactiveGradient::Dimmed,
            progress_speed: 6.0,

            bar_active: "━".to_string(),
            bar_inactive: "─".to_string(),
            waveform_style: Marker::Braille,
            oscilloscope_style: Marker::Braille,

            decorator: Rc::from("✧".to_string()),
        }
    }
}
