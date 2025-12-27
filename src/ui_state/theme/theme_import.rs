use ratatui::{style::Color, widgets::BorderType};
use serde::{Deserialize, Deserializer};
use std::str::FromStr;

#[derive(Deserialize)]
pub struct ThemeImport {
    pub colors: ColorScheme,
    pub borders: BorderScheme,
    #[serde(default = "default_extras")]
    pub extras: ExtraScheme,
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

    pub progress: ProgressGradientRaw,

    #[serde(default = "default_inactive")]
    pub progress_i: ProgressGradientRaw,

    #[serde(default = "default_speed")]
    pub progress_speed: f32,
}

#[derive(Deserialize)]
pub struct BorderScheme {
    pub border_display: String,
    #[serde(deserialize_with = "deserialize_border_type")]
    pub border_type: BorderType,
}

#[derive(Deserialize)]
pub struct ExtraScheme {
    #[serde(default = "default_dark")]
    pub is_dark: bool,
    #[serde(default = "default_decorator")]
    pub decorator: String,
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

const DEFAULT_SPEED: f32 = 6.0;
fn default_speed() -> f32 {
    DEFAULT_SPEED
}

const DECORATOR: &str = "✧";
fn default_decorator() -> String {
    DECORATOR.to_string()
}

fn default_dark() -> bool {
    true
}

fn default_extras() -> ExtraScheme {
    ExtraScheme {
        is_dark: default_dark(),
        decorator: default_decorator(),
    }
}

// Allows for case-insenstive matching
fn deserialize_border_type<'de, D>(deserializer: D) -> Result<BorderType, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    // Remove common separators and compare lowercase
    let normalized: String = s
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect();

    match normalized.as_str() {
        "plain" => Ok(BorderType::Plain),
        "rounded" => Ok(BorderType::Rounded),
        "double" => Ok(BorderType::Double),
        "thick" => Ok(BorderType::Thick),
        "lightdoubledashed" => Ok(BorderType::LightDoubleDashed),
        "heavydoubledashed" => Ok(BorderType::HeavyDoubleDashed),
        "lighttripledashed" => Ok(BorderType::LightTripleDashed),
        "heavytripledashed" => Ok(BorderType::HeavyTripleDashed),
        "lightquadrupledashed" => Ok(BorderType::LightQuadrupleDashed),
        "heavyquadrupledashed" => Ok(BorderType::HeavyQuadrupleDashed),
        "quadrantinside" => Ok(BorderType::QuadrantInside),
        "quadrantoutside" => Ok(BorderType::QuadrantOutside),
        _ => Err(serde::de::Error::custom("Invalid variant")),
    }
}
