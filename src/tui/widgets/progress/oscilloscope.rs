use crate::ui_state::{Pane, UiState};
use ratatui::{
    style::Stylize,
    widgets::{
        canvas::{Canvas, Context, Line},
        Block, Padding, StatefulWidget, Widget,
    },
};

pub struct Oscilloscope;

impl StatefulWidget for Oscilloscope {
    type State = UiState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let theme = state.get_theme(&Pane::Popup);
        let samples = state.get_oscilloscope_data();

        if samples.is_empty() {
            return;
        }

        let v_marg = match area.height > 20 {
            true => ((area.height as f32) * 0.1) as u16,
            false => 0,
        };

        let elapsed = state.get_playback_elapsed().as_secs_f32();

        Canvas::default()
            .x_bounds([0.0, samples.len() as f64])
            .y_bounds([-1.0, 1.0])
            .paint(|ctx| {
                draw_vibrant_gradient(ctx, &samples, elapsed);
            })
            .background_color(theme.bg_p)
            .block(Block::new().bg(theme.bg_p).padding(Padding {
                left: 1,
                right: 1,
                top: v_marg,
                bottom: v_marg,
            }))
            .render(area, buf);
    }
}

// fn draw_wave(ctx: &mut Context, samples: &[f32], color: Color) {
//     for (i, window) in samples.windows(2).enumerate() {
//         let x1 = i as f64;
//         let y1 = window[0] as f64;
//         let x2 = (i + 1) as f64;
//         let y2 = window[1] as f64;
//
//         ctx.draw(&Line {
//             x1,
//             y1,
//             x2,
//             y2,
//             color,
//         });
//     }
// }

fn draw_vibrant_gradient(ctx: &mut Context, samples: &[f32], time: f32) {
    let intensity: f32 = samples.iter().map(|s| s.abs()).sum::<f32>() / samples.len() as f32;
    let boosted = (intensity * 3.0).min(1.0); // Aggressive boost

    for (i, window) in samples.windows(2).enumerate() {
        let x1 = i as f64;
        let y1 = window[0] as f64;
        let x2 = (i + 1) as f64;
        let y2 = window[1] as f64;

        let progress = i as f32 / samples.len() as f32;

        // Always maximum saturation, only brightness varies
        let hue = (progress * 360.0 + time * 30.0) % 360.0;
        let saturation = 1.0; // Always max saturation
        let value = 0.7 + (boosted * 0.3); // 0.7 to 1.0

        let color = super::hsv_to_rgb(hue, saturation, value);

        ctx.draw(&Line {
            x1,
            y1,
            x2,
            y2,
            color,
        });
    }
}
