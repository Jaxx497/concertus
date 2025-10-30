mod oscilloscope;
mod progress_bar;
mod timer;
mod waveform;

use crate::{
    tui::widgets::progress::{
        oscilloscope::Oscilloscope, progress_bar::ProgressBar, timer::Timer, waveform::Waveform,
    },
    ui_state::{ProgressDisplay, ProgressGradient, UiState},
};
use ratatui::{style::Color, widgets::StatefulWidget};

pub(crate) const SHARP_FACTOR: f32 = 0.5;

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

fn get_gradient_color(gradient: &ProgressGradient, position: f32, time: f32) -> Color {
    match gradient {
        ProgressGradient::Static(color) => *color,
        ProgressGradient::Gradient(colors) => {
            if colors.is_empty() {
                return Color::Reset;
            }
            if colors.len() == 1 {
                return colors[0];
            }

            let t = ((position + time * 0.2) % 1.0).abs();

            let segment_count = colors.len();
            let segment_f = t * segment_count as f32;
            let segment = (segment_f as usize).min(segment_count - 1);
            let local_t = segment_f - segment as f32;

            let next_segment = (segment + 1) % segment_count;

            let sharp = sharpen_interpolation(local_t, SHARP_FACTOR);

            interpolate_color(colors[segment], colors[next_segment], sharp)
        }
    }
}

pub(crate) fn interpolate_color(c1: Color, c2: Color, t: f32) -> Color {
    match (c1, c2) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => Color::Rgb(
            (r1 as f32 + (r2 as f32 - r1 as f32) * t) as u8,
            (g1 as f32 + (g2 as f32 - g1 as f32) * t) as u8,
            (b1 as f32 + (b2 as f32 - b1 as f32) * t) as u8,
        ),
        _ => c1,
    }
}

pub(crate) fn sharpen_interpolation(t: f32, power: f32) -> f32 {
    if t < 0.5 {
        (t * 2.0).powf(power) / 2.0
    } else {
        1.0 - ((1.0 - t) * 2.0).powf(power) / 2.0
    }
}
