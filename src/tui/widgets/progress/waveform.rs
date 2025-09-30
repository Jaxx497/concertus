use crate::{
    domain::SongInfo,
    tui::widgets::WAVEFORM_WIDGET_HEIGHT,
    ui_state::{Pane, UiState},
};
use canvas::Context;
use ratatui::{
    style::{Color, Stylize},
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
        let theme = state.get_theme(&Pane::TrackList);

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

                let progress = elapsed.as_secs_f32() / duration_f32;
                let line_mode = area.width < 170;

                for (idx, amp) in waveform.iter().enumerate() {
                    let hgt = (*amp as f64 * WAVEFORM_WIDGET_HEIGHT).round();
                    let color = match (idx as f32 / wf_len as f32) < progress {
                        true => theme.progress_complete,
                        false => theme.progress_incomplete,
                    };

                    match line_mode {
                        true => draw_waveform_line(ctx, idx as f64, hgt, color),
                        false => draw_waveform_rect(ctx, idx as f64, hgt, color),
                    }
                }
            })
            .background_color(theme.bg_global)
            .block(Block::new().bg(theme.bg_global).padding(Padding {
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
