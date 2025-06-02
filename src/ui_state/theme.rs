use ratatui::{style::Color, widgets::Borders};

const DARK_WHITE: Color = Color::Rgb(210, 210, 210);
const MID_GRAY: Color = Color::Rgb(100, 100, 100);
const DARK_GRAY: Color = Color::Rgb(25, 25, 25);
const DARK_GRAY_FADED: Color = Color::Rgb(10, 10, 10);
pub static GOOD_RED: Color = Color::Rgb(255, 70, 70);
const GOLD: Color = Color::Rgb(220, 220, 100);
// const GOLD: Color = Color::Rgb(255, 200, 20);

pub struct DisplayTheme {
    pub bg: Color,
    pub border: Color,
    pub border_display: Borders,
    pub text_focused: Color,
    pub text_secondary: Color,
    pub text_faded: Color,
    pub text_highlighted: Color,
}

pub(crate) struct Theme {
    pub bg_focused: Color,
    pub bg_unfocused: Color,
    pub border_focused: Color,
    pub border_unfocused: Color,
    pub text_focused: Color,
    pub text_secondary: Color,
    pub text_unfocused: Color,
    pub text_highlighted: Color,
}

impl Theme {
    pub fn set_generic_theme() -> Theme {
        Theme {
            bg_focused: DARK_GRAY,
            bg_unfocused: DARK_GRAY_FADED,

            text_focused: DARK_WHITE,
            text_unfocused: MID_GRAY,
            text_secondary: GOOD_RED,
            text_highlighted: GOLD,

            border_focused: GOOD_RED,
            ..Default::default()
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            bg_focused: Color::default(),
            bg_unfocused: Color::default(),
            border_focused: Color::default(),
            border_unfocused: Color::default(),
            text_focused: Color::default(),
            text_secondary: Color::default(),
            text_unfocused: Color::default(),
            text_highlighted: Color::default(),
        }
    }
}
