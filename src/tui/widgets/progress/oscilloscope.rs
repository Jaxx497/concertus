use crate::ui_state::UiState;
use ratatui::{
    style::{Color, Stylize},
    widgets::{
        Block, Padding, StatefulWidget, Widget,
        canvas::{Canvas, Context, Line},
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
        let samples = state.get_oscilloscope_data();

        if samples.is_empty() {
            return;
        }

        Canvas::default()
            .x_bounds([0.0, samples.len() as f64])
            .y_bounds([-1.0, 1.0])
            .paint(|ctx| {
                draw_waveform(ctx, &samples, state.theme.progress_complete);
            })
            .background_color(state.theme.bg_unfocused)
            .block(Block::new().bg(state.theme.bg_unfocused).padding(Padding {
                left: 2,
                right: 2,
                top: 0,
                bottom: 0,
            }))
            .render(area, buf);
    }
}

fn draw_waveform(ctx: &mut Context, samples: &[f32], color: Color) {
    for (i, window) in samples.windows(2).enumerate() {
        let x1 = i as f64;
        let y1 = window[0] as f64;
        let x2 = (i + 1) as f64;
        let y2 = window[1] as f64;

        ctx.draw(&Line {
            x1,
            y1,
            x2,
            y2,
            color,
        });
    }
}
