use ratatui::widgets::BorderType;
use serde::{Deserialize, Deserializer};

#[derive(Deserialize)]
pub struct BorderScheme {
    #[serde(default = "default_borders")]
    pub border_display: String,
    #[serde(
        default = "default_border_type",
        deserialize_with = "deserialize_border_type"
    )]
    pub border_type: BorderType,
}

pub fn default_border_scheme() -> BorderScheme {
    BorderScheme {
        border_display: default_borders(),
        border_type: default_border_type(),
    }
}

const DEFAULT_BORDERS: &str = "all";
fn default_borders() -> String {
    DEFAULT_BORDERS.to_string()
}

fn default_border_type() -> BorderType {
    BorderType::Rounded
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
        _ => Ok(BorderType::Rounded),
    }
}
