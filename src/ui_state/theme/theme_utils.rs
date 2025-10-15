use anyhow::{Result, anyhow, bail};
use ratatui::{
    style::Color,
    widgets::{BorderType, Borders},
};

pub fn parse_color(s: &str) -> Result<Color> {
    match s {
        s if s.starts_with('#') => parse_hex(s),
        s if s.starts_with("rgb(") => parse_rgb(s),
        _ => try_from_str(s.trim()),
    }
}

pub fn parse_hex(s: &str) -> Result<Color> {
    let hex = s.trim_start_matches('#');
    if hex.len() != 6 {
        bail!("Invalid hex input: {s}\nExpected format\"#FF20D5\"");
    }

    let r = u8::from_str_radix(&hex[0..2], 16)?;
    let g = u8::from_str_radix(&hex[2..4], 16)?;
    let b = u8::from_str_radix(&hex[4..], 16)?;

    Ok(Color::Rgb(r, g, b))
}

pub fn parse_rgb(s: &str) -> Result<Color> {
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

pub fn try_from_str(s: &str) -> Result<Color> {
    match s.to_lowercase().as_str() {
        "" | "none" => Ok(Color::default()),
        "black" => Ok(Color::Black),
        "red" => Ok(Color::Red),
        "green" => Ok(Color::Green),
        "yellow" => Ok(Color::Yellow),
        "blue" => Ok(Color::Blue),
        "magenta" => Ok(Color::Magenta),
        "cyan" => Ok(Color::Cyan),
        "white" => Ok(Color::White),
        "gray" | "grey" => Ok(Color::Gray),
        "darkgray" | "darkgrey" => Ok(Color::DarkGray),
        "lightred" => Ok(Color::LightRed),
        "lightgreen" => Ok(Color::LightGreen),
        "lightyellow" => Ok(Color::LightYellow),
        "lightblue" => Ok(Color::LightBlue),
        "lightmagenta" => Ok(Color::LightMagenta),
        "lightcyan" => Ok(Color::LightCyan),
        _ => Err(anyhow!("Invalid input: {}", s)),
    }
}

pub fn parse_border_type(s: &str) -> BorderType {
    match s.trim().to_lowercase().as_str() {
        "plain" => BorderType::Plain,
        "double" => BorderType::Double,
        "thick" => BorderType::Thick,
        _ => BorderType::Rounded,
    }
}

pub fn parse_borders(s: &str) -> Borders {
    match s.to_lowercase().trim() {
        "" | "none" => Borders::NONE,
        _ => Borders::ALL,
    }
}
