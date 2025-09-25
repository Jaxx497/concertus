// In src/tui/widgets/oscilloscope.rs
use ratatui::{
    layout::Rect,
    style::{Color, Stylize},
    text::Text,
    widgets::{
        Block, Padding, StatefulWidget, Widget,
        canvas::{Canvas, Context, Line},
    },
};

use crate::{
    tui::widgets::DUR_WIDTH,
    ui_state::{GOOD_RED_DARK, UiState},
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

        let x_duration = area.width - 8;
        let y = buf.area().height
            - match area.height {
                0 => 1,
                _ => area.height / 2 - 1,
            };

        let player_state = state.playback.player_state.lock().unwrap();
        let elapsed_str = player_state.elapsed_display.as_str();
        let duration_str = player_state.duration_display.as_str();

        Text::from(elapsed_str)
            .fg(Color::DarkGray)
            .right_aligned()
            .render(Rect::new(2, y, DUR_WIDTH, 1), buf);

        Text::from(duration_str)
            .fg(Color::DarkGray)
            .right_aligned()
            .render(Rect::new(x_duration, y, DUR_WIDTH, 1), buf);

        Canvas::default()
            .x_bounds([0.0, samples.len() as f64])
            .y_bounds([-1.0, 1.0])
            .paint(|ctx| {
                draw_waveform(ctx, &samples);
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

fn draw_waveform(ctx: &mut Context, samples: &[f32]) {
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
            color: GOOD_RED_DARK,
        });
    }
}
