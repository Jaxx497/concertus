mod oscilloscope;
mod progress_bar;
mod timer;
mod waveform;
use ratatui::{style::Color, widgets::StatefulWidget};

use crate::{
    tui::widgets::progress::{
        oscilloscope::Oscilloscope, progress_bar::ProgressBar, timer::Timer, waveform::Waveform,
    },
    ui_state::{ProgressDisplay, UiState},
};

pub struct Progress;
impl StatefulWidget for Progress {
    type State = UiState;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        if state.get_now_playing().is_some() {
            Timer.render(area, buf, state);
            match &state.get_progress_display() {
                ProgressDisplay::ProgressBar => ProgressBar.render(area, buf, state),
                ProgressDisplay::Waveform => match state.waveform_is_valid() {
                    true => Waveform.render(area, buf, state),
                    false => Oscilloscope.render(area, buf, state),
                },
                ProgressDisplay::Oscilloscope => Oscilloscope.render(area, buf, state),
            }
        }
    }
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Color {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = match h as u16 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    Color::Rgb(
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}
