use ratatui::style::Color;
use serde::{Deserialize, Deserializer};
use std::str::FromStr;

#[derive(Deserialize)]
pub struct ThemeImport {
    pub colors: ColorScheme,
    pub borders: BorderScheme,
}

#[derive(Deserialize)]
pub struct ColorScheme {
    pub dark: bool,
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

    pub progress: ProgressGradientRaw,

    #[serde(default = "default_inactive")]
    pub progress_i: ProgressGradientRaw,

    #[serde(default = "default_speed")]
    pub progress_speed: f32,
}

#[derive(Deserialize)]
pub struct BorderScheme {
    pub border_display: String,
    // pub border_display: Borders,
    pub border_type: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum ProgressGradientRaw {
    Single(String),
    Gradient(Vec<String>),
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

fn default_inactive() -> ProgressGradientRaw {
    ProgressGradientRaw::Single("dimmed".to_string())
}

const DEFAULT_SPEED: f32 = 8.0;
const fn default_speed() -> f32 {
    DEFAULT_SPEED
}
