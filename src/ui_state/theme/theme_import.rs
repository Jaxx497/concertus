use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct ThemeImport {
    pub name: String,

    pub colors: ColorScheme,
    pub borders: BorderScheme,
}

#[derive(Default, Serialize, Deserialize)]
pub struct ColorScheme {
    pub bg_focused: String,
    pub bg_unfocused: String,
    pub bg_progress: String,

    pub text_focused: String,
    pub text_unfocused: String,
    pub text_secondary: String,
    pub text_secondary_u: String,
    pub text_highlight: String,

    pub highlight: String,
    pub highlight_u: String,

    pub border_focused: String,
    pub border_unfocused: String,

    pub progress_complete: String,
    pub progress_incomplete: String,
}

#[derive(Default, Serialize, Deserialize)]
pub struct BorderScheme {
    pub border_display: String,
    pub border_type: String,
}
