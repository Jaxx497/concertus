use crate::{
    domain::SongInfo,
    tui::widgets::WAVEFORM_WIDGET_HEIGHT,
    ui_state::{Pane, UiState},
};
use ratatui::{
    style::{Color, Stylize},
    widgets::{
        canvas::{Canvas, Context, Line, Rectangle},
        Block, Padding, StatefulWidget, Widget,
    },
};

pub struct Waveform;
impl StatefulWidget for Waveform {
    type State = UiState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let theme = state.get_theme(&Pane::TrackList);

        let height = area.height as f32;

        let normalized = ((height - 6.0) / 20.0).max(0.0);
        let smoothed = normalized / (1.0 + normalized);
        let padding_vertical = (smoothed * height * 0.45) as u16;

        let padding = Padding {
            left: 10,
            right: 10,
            top: match area.height > 6 {
                true => padding_vertical,
                false => 1,
            },
            bottom: padding_vertical,
        };

        let np = state
            .get_now_playing()
            .expect("Expected a song to be playing. [Widget: Waveform]");

        let waveform = state.get_waveform_visual().to_vec();
        let wf_len = waveform.len();

        Canvas::default()
            .x_bounds([0.0, wf_len as f64])
            .y_bounds([WAVEFORM_WIDGET_HEIGHT * -1.0, WAVEFORM_WIDGET_HEIGHT])
            .paint(|ctx| {
                let duration_f32 = &np.get_duration_f32();
                let elapsed = &state.get_playback_elapsed();

                let elapsed_secs = elapsed.as_secs_f32();
                let progress = elapsed_secs / duration_f32;

                let line_mode = area.width < 170;

                for (idx, amp) in waveform.iter().enumerate() {
                    let hgt = (*amp as f64 * WAVEFORM_WIDGET_HEIGHT).round();
                    let position = idx as f32 / wf_len as f32;

                    let color = if position < progress {
                        get_vibrant_color(position, elapsed_secs)
                    } else {
                        get_unplayed_color(position, *amp)
                    };

                    match line_mode {
                        true => draw_waveform_line(ctx, idx as f64, hgt, color),
                        false => draw_waveform_rect(ctx, idx as f64, hgt, color),
                    }
                }
            })
            .background_color(theme.bg_p)
            .block(Block::new().bg(theme.bg_p).padding(padding))
            .render(area, buf)
    }
}

/// Lines create a more detailed and cleaner look
/// especially when seen in smaller windows
fn draw_waveform_line(ctx: &mut Context, idx: f64, hgt: f64, color: Color) {
    ctx.draw(&Line {
        x1: idx,
        x2: idx,
        y1: hgt,
        y2: hgt * -1.0,
        color,
    })
}

/// Rectangles cleanly extend the waveform when in
/// full-screen view
fn draw_waveform_rect(ctx: &mut Context, idx: f64, hgt: f64, color: Color) {
    ctx.draw(&Rectangle {
        x: idx,
        y: hgt * -1.0,
        width: 0.5,        // This makes the waveform cleaner on resize
        height: hgt * 2.0, // Rectangles are drawn from the bottom
        color,
    });
}

fn get_vibrant_color(position: f32, time: f32) -> Color {
    let h = (position * 360.0 + time * 300.0) % 360.0;
    let s = 1.0;
    let v = 0.9;

    super::hsv_to_rgb(h, s, v)
}

fn get_unplayed_color(position: f32, amplitude: f32) -> Color {
    let h = (position * 360.0) % 360.0;
    let s = 0.4;
    let v = 0.3 + (amplitude * 0.15);
    super::hsv_to_rgb(h, s, v)
}
