use serde::Deserialize;

#[derive(Deserialize)]
pub struct ExtraScheme {
    #[serde(default = "default_dark")]
    pub is_dark: bool,
    #[serde(default = "default_decorator")]
    pub decorator: String,
}

pub fn default_extras() -> ExtraScheme {
    ExtraScheme {
        is_dark: default_dark(),
        decorator: default_decorator(),
    }
}

fn default_dark() -> bool {
    true
}

const DECORATOR: &str = "✧";
fn default_decorator() -> String {
    DECORATOR.to_string()
}
