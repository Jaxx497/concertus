use serde::Deserialize;

#[derive(Deserialize)]
pub struct ProgressScheme {
    pub elapsed: ProgressGradientRaw,
    #[serde(default = "default_unplayed")]
    pub unplayed: ProgressGradientRaw,

    #[serde(default = "default_speed")]
    pub speed: f32,

    #[serde(default = "default_bar_active")]
    pub bar_elapsed: String,
    #[serde(default = "default_bar_inactive")]
    pub bar_unplayed: String,

    #[serde(default = "default_display_style")]
    pub waveform_style: String,
    #[serde(default = "default_display_style")]
    pub oscilloscope_style: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum ProgressGradientRaw {
    Single(String),
    Gradient(Vec<String>),
}

const DEFAULT_UNPLAYED: &str = "dimmed";
fn default_unplayed() -> ProgressGradientRaw {
    ProgressGradientRaw::Single(DEFAULT_UNPLAYED.to_string())
}

const DEFAULT_SPEED: f32 = 6.0;
fn default_speed() -> f32 {
    DEFAULT_SPEED
}

const DEFAULT_BAR_ACTIVE: &str = "━";
fn default_bar_active() -> String {
    DEFAULT_BAR_ACTIVE.to_string()
}

const DEFAULT_BAR_INACTIVE: &str = "─";
fn default_bar_inactive() -> String {
    DEFAULT_BAR_INACTIVE.to_string()
}

const DEFAULT_DISPLAY_STYLE: &str = "dots";
fn default_display_style() -> String {
    DEFAULT_DISPLAY_STYLE.to_string()
}
