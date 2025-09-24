use crate::{
    domain::SongInfo,
    tui::widgets::{DUR_WIDTH, WAVEFORM_WIDGET_HEIGHT},
    ui_state::UiState,
};
use canvas::Context;
use ratatui::{
    layout::Rect,
    style::{Color, Stylize},
    text::Text,
    widgets::{
        StatefulWidget,
        canvas::{Canvas, Rectangle},
        *,
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
        let np = state
            .get_now_playing()
            .expect("Expected a song to be playing. [Widget: Waveform]");

        let waveform = state.get_waveform_visual().to_vec();
        let wf_len = waveform.len();

        let x_duration = area.width - 8;
        let y = buf.area().height
            - match area.height {
                0 => 1,
                _ => area.height / 2 + 1,
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

        // PREVENT DEADLOCKS
        drop(player_state);

        Canvas::default()
            .x_bounds([0.0, wf_len as f64])
            .y_bounds([WAVEFORM_WIDGET_HEIGHT * -1.0, WAVEFORM_WIDGET_HEIGHT])
            .paint(|ctx| {
                let duration_f32 = &np.get_duration_f32();
                let elapsed = &state.get_playback_elapsed();

                let progress = elapsed.as_secs_f32() / duration_f32;
                let line_mode = area.width < 170;

                for (idx, amp) in waveform.iter().enumerate() {
                    let hgt = (*amp as f64 * WAVEFORM_WIDGET_HEIGHT).round();
                    let color = match (idx as f32 / wf_len as f32) < progress {
                        true => Color::Rgb(170, 0, 170),
                        false => Color::default(),
                    };

                    match line_mode {
                        true => draw_waveform_line(ctx, idx as f64, hgt, color),
                        false => draw_waveform_rect(ctx, idx as f64, hgt, color),
                    }
                }
            })
            .background_color(state.theme.bg_unfocused)
            .block(Block::new().bg(state.theme.bg_unfocused).padding(Padding {
                left: 10,
                right: 10,
                top: 1,
                bottom: 0,
            }))
            .render(area, buf)
    }
}

/// Lines create a more detailed and cleaner look
/// especially when seen in smaller windows
fn draw_waveform_line(ctx: &mut Context, idx: f64, hgt: f64, color: Color) {
    ctx.draw(&canvas::Line {
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
        x: idx as f64,
        y: hgt * -1.0,
        width: f64::from(0.5), // This value makes the waveform cleaner on resize
        height: hgt * 2.0,
        color,
    });
}
