use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct ThemeImport {
    pub name: String,
    pub colors: ColorScheme,
    pub borders: BorderScheme,
}

#[derive(Default, Serialize, Deserialize)]
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
}

#[derive(Default, Serialize, Deserialize)]
pub struct BorderScheme {
    pub border_display: String,
    pub border_type: String,
}
