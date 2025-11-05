use ratatui::style::Color;

pub const SHARP_FACTOR: f32 = 2.0;

pub fn get_gradient_color(gradient: &[Color], position: f32, time: f32) -> Color {
    if gradient.is_empty() {
        return Color::Reset;
    }

    let t = ((position + time) % 1.0).abs();

    let segment_count = gradient.len();
    let segment_f = t * segment_count as f32;
    let segment = (segment_f as usize).min(segment_count - 1);
    let local_t = segment_f - segment as f32;

    let next_segment = (segment + 1) % segment_count;
    let sharp = sharpen_interpolation(local_t, SHARP_FACTOR);

    interpolate_color(gradient[segment], gradient[next_segment], sharp)
}

fn interpolate_color(c1: Color, c2: Color, t: f32) -> Color {
    match (c1, c2) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => Color::Rgb(
            (r1 as f32 + (r2 as f32 - r1 as f32) * t) as u8,
            (g1 as f32 + (g2 as f32 - g1 as f32) * t) as u8,
            (b1 as f32 + (b2 as f32 - b1 as f32) * t) as u8,
        ),
        _ => c1,
    }
}

fn sharpen_interpolation(t: f32, power: f32) -> f32 {
    if t < 0.5 {
        (t * 2.0).powf(power) / 2.0
    } else {
        1.0 - ((1.0 - t) * 2.0).powf(power) / 2.0
    }
}

pub fn fade_color(is_dark: bool, color: Color, factor: f32) -> Color {
    match is_dark {
        true => dim_color(color, factor),
        false => brighten_color(color, factor),
    }
}

fn dim_color(color: Color, factor: f32) -> Color {
    match color {
        Color::Rgb(r, g, b) => Color::Rgb(
            (r as f32 * factor) as u8,
            (g as f32 * factor) as u8,
            (b as f32 * factor) as u8,
        ),
        other => other,
    }
}

fn brighten_color(color: Color, factor: f32) -> Color {
    let factor = 1.0 - factor;
    match color {
        Color::Rgb(r, g, b) => Color::Rgb(
            (r as f32 + (255.0 - r as f32) * factor) as u8,
            (g as f32 + (255.0 - g as f32) * factor) as u8,
            (b as f32 + (255.0 - b as f32) * factor) as u8,
        ),
        other => other,
    }
}
