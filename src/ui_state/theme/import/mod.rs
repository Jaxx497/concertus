use ratatui::style::Color;
use serde::{Deserialize, Deserializer};
use std::str::FromStr;

mod borders;
mod extras;
mod progress;

pub use borders::default_border_scheme;
pub use extras::default_extras;
pub use progress::ProgressGradientRaw;

#[derive(Deserialize)]
pub struct ThemeImport {
    pub colors: ColorScheme,
    #[serde(default = "default_border_scheme")]
    pub borders: borders::BorderScheme,

    pub progress: progress::ProgressScheme,

    #[serde(default = "default_extras")]
    pub extras: extras::ExtraScheme,
}

#[derive(Deserialize)]
pub struct ColorScheme {
    pub surface_global: ThemeColor,
    pub surface_active: ThemeColor,
    pub surface_inactive: ThemeColor,
    pub surface_error: ThemeColor,

    // Text colors
    pub text_primary: ThemeColor,
    pub text_secondary: ThemeColor,
    pub text_secondary_in: ThemeColor,
    pub text_selection: ThemeColor,
    pub text_muted: ThemeColor,

    // Border colors
    pub border_active: ThemeColor,
    pub border_inactive: ThemeColor,

    // Accent
    pub accent: ThemeColor,
    pub accent_inactive: ThemeColor,

    // Selection colors
    pub selection: ThemeColor,
    pub selection_inactive: ThemeColor,
}

#[derive(Clone, Copy, Debug)]
pub struct ThemeColor(pub Color);

impl<'de> Deserialize<'de> for ThemeColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        // Handle transparent
        match s.to_lowercase().as_str() {
            "" | "none" => return Ok(ThemeColor(Color::Reset)),
            _ => {}
        }

        Color::from_str(&s)
            .map(ThemeColor)
            .map_err(serde::de::Error::custom)
    }
}

impl std::ops::Deref for ThemeColor {
    type Target = Color;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<ThemeColor> for Color {
    fn from(tc: ThemeColor) -> Self {
        tc.0
    }
}
