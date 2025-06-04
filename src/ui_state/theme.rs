use ratatui::{style::Color, widgets::Borders};

const DARK_WHITE: Color = Color::Rgb(210, 210, 210);
const MID_GRAY: Color = Color::Rgb(100, 100, 100);
const DARK_GRAY: Color = Color::Rgb(25, 25, 25);
const DARK_GRAY_FADED: Color = Color::Rgb(10, 10, 10);
pub const GOOD_RED: Color = Color::Rgb(255, 70, 70);
pub const GOOD_RED_DARK: Color = Color::Rgb(180, 50, 50);
pub const GOLD: Color = Color::Rgb(220, 220, 100);
pub const GOLD_FADED: Color = Color::Rgb(130, 130, 60);

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
    pub text_secondary_u: Color,
    pub text_unfocused: Color,
    pub text_highlighted: Color,
    pub text_highlighted_u: Color,
}

impl Theme {
    pub fn set_generic_theme() -> Theme {
        Theme {
            bg_focused: DARK_GRAY,
            bg_unfocused: DARK_GRAY_FADED,

            text_focused: DARK_WHITE,
            text_unfocused: MID_GRAY,
            text_secondary: GOOD_RED,
            text_secondary_u: GOOD_RED_DARK,
            text_highlighted: GOLD,
            text_highlighted_u: GOLD_FADED,

            border_focused: GOLD,
            border_unfocused: Color::Rgb(50, 50, 50),
        }
    }
}
