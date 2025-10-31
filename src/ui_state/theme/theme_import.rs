use serde::Deserialize;

#[derive(Deserialize)]
pub struct ThemeImport {
    pub colors: ColorScheme,
    pub borders: BorderScheme,
}

#[derive(Deserialize)]
pub struct ColorScheme {
    pub surface_global: String,
    pub surface_active: String,
    pub surface_inactive: String,
    pub surface_error: String,

    // Text colors
    pub text_primary: String,
    pub text_secondary: String,
    pub text_secondary_in: String,
    pub text_selection: String,
    pub text_muted: String,

    // Border colors
    pub border_active: String,
    pub border_inactive: String,

    // Accent
    pub accent: String,
    pub accent_inactive: String,

    // Selection colors
    pub selection: String,
    pub selection_inactive: String,

    pub waveform: ProgressGradientRaw,
    pub waveform_i: ProgressGradientRaw,

    pub oscilloscope: ProgressGradientRaw,
}

#[derive(Deserialize)]
pub struct BorderScheme {
    pub border_display: String,
    pub border_type: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum ProgressGradientRaw {
    Single(String),
    Gradient(Vec<String>),
}
