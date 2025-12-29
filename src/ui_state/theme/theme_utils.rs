use std::str::FromStr;

use anyhow::{Result, anyhow};
use ratatui::{style::Color, symbols::Marker, widgets::Borders};

pub(super) fn parse_color(s: &str) -> Result<Color> {
    match s {
        s if s.starts_with("rgb(") => parse_rgb(s),
        _ => Ok(Color::from_str(s)?),
    }
}

pub(super) fn parse_rgb(s: &str) -> Result<Color> {
    if s.ends_with(')') {
        let inner = &s[4..s.len() - 1];
        let parts = inner.split(',').collect::<Vec<&str>>();
        if parts.len() == 3 {
            let r = parts[0].trim().parse::<u8>()?;
            let g = parts[1].trim().parse::<u8>()?;
            let b = parts[2].trim().parse::<u8>()?;
            return Ok(Color::Rgb(r, g, b));
        }
    }
    Err(anyhow!(
        "Invalid rgb input: {s}\nExpected ex: \"rgb(255, 50, 120)\""
    ))
}

pub(super) fn parse_borders(s: &str) -> Borders {
    match s.to_lowercase().trim() {
        "" | "none" => Borders::NONE,
        _ => Borders::ALL,
    }
}

pub(super) fn parse_display(s: &str) -> Marker {
    use Marker::*;
    match s.to_lowercase().trim() {
        "block2" | "halfblock" => HalfBlock,
        "block4" | "quadrant" => Quadrant,
        "block6" | "sextant" => Sextant,
        "blocks" | "blocky" | "block8" | "octant" => Octant,
        _ => Braille,
    }
}
