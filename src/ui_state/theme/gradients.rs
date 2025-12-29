use crate::ui_state::{ProgressGradientRaw, theme::theme_utils::parse_color};
use anyhow::Result;
use ratatui::style::Color;
use std::sync::Arc;

#[derive(Clone)]
pub enum ProgressGradient {
    Static(Color),
    Gradient(Arc<[Color]>),
}

#[derive(Clone)]
pub enum InactiveGradient {
    Dimmed,
    Still,
    Static(Color),
    Gradient(Arc<[Color]>),
}

impl ProgressGradient {
    pub(super) fn from_raw(raw: &ProgressGradientRaw) -> Result<ProgressGradient> {
        match raw {
            ProgressGradientRaw::Single(c) => Ok(ProgressGradient::Static(parse_color(&c)?)),
            ProgressGradientRaw::Gradient(colors) => {
                if colors.len() == 1 {
                    return Ok(ProgressGradient::Static(parse_color(&colors[0])?));
                }

                let gradient = colors
                    .iter()
                    .map(|c| parse_color(&c))
                    .collect::<Result<Vec<Color>>>()?;

                Ok(ProgressGradient::Gradient(gradient.into()))
            }
        }
    }
}

impl InactiveGradient {
    pub(super) fn from_raw(raw: &ProgressGradientRaw) -> Result<InactiveGradient> {
        match raw {
            ProgressGradientRaw::Single(s) if s.to_lowercase().as_str() == "dimmed" => {
                Ok(InactiveGradient::Dimmed)
            }
            ProgressGradientRaw::Single(s) if s.to_lowercase().as_str() == "still" => {
                Ok(InactiveGradient::Still)
            }
            ProgressGradientRaw::Single(s) => {
                let color = parse_color(&s)?;
                Ok(InactiveGradient::Static(color))
            }
            ProgressGradientRaw::Gradient(colors) => {
                let gradient = colors
                    .iter()
                    .map(|c| parse_color(&c))
                    .collect::<Result<Vec<Color>>>()?;

                Ok(InactiveGradient::Gradient(gradient.into()))
            }
        }
    }
}
