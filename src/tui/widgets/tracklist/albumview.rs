use crate::{
    domain::{SimpleSong, SongInfo},
    get_readable_duration, truncate_at_last_space,
    ui_state::{DisplayTheme, Pane, UiState},
    DurationStyle,
};
use ratatui::{
    layout::{Alignment, Flex},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{StatefulWidget, *},
};
use std::sync::Arc;
use std::{
    collections::{HashMap, HashSet},
    sync::LazyLock,
};

use super::{get_header, get_widths, COLUMN_SPACING, PADDING, SELECTOR};

static SUPERSCRIPT: std::sync::LazyLock<std::collections::HashMap<u32, &str>> =
    LazyLock::new(|| {
        HashMap::from([
            (0, "⁰"),
            (1, "¹"),
            (2, "²"),
            (3, "³"),
            (4, "⁴"),
            (5, "⁵"),
            (6, "⁶"),
            (7, "⁷"),
            (8, "⁸"),
            (9, "⁹"),
        ])
    });

pub struct AlbumView;
impl StatefulWidget for AlbumView {
    type State = UiState;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let album_list = &state.filtered_albums;
        if album_list.is_empty() {
            return;
        }

        let theme = &state.get_theme(&Pane::TrackList);

        let album_idx = state.album_pos.selected().unwrap_or(0);
        let album = &album_list[album_idx];
        let album_title = truncate_at_last_space(&album.title, (area.width / 3) as usize);

        let queued_ids: HashSet<u64> = state.queue.iter().map(|s| s.get_id()).collect();
        let now_playing_id = state.get_now_playing().map(|s| s.id);

        let disc_count = album
            .tracklist
            .iter()
            .filter_map(|s| s.disc_no)
            .max()
            .unwrap_or(1) as usize;

        let rows = album
            .tracklist
            .iter()
            .map(|song| {
                let is_queued = queued_ids.contains(&song.id);
                let is_playing = now_playing_id == Some(song.id);

                let title_cell = match (is_queued, is_playing) {
                    (true, false) => Line::from_iter([
                        song.get_title().fg(theme.text_focused),
                        " [queued]".fg(theme.text_faded).italic().into(),
                    ]),
                    (false, true) => Line::from_iter([
                        song.get_title().fg(theme.text_focused),
                        " ♫".fg(theme.text_secondary).into(),
                    ]),
                    _ => Line::from_iter([song.get_title().fg(theme.text_focused)]),
                };

                let track_no_cell = get_track_discs(song, disc_count, theme);
                let artist_cell = Cell::from(song.get_artist()).fg(theme.text_focused);
                let format = Cell::from(format!("{}", song.format)).fg(theme.text_secondary);
                let duration_str = get_readable_duration(song.duration, DurationStyle::Clean);
                let duration_cell =
                    Cell::from(Text::from(duration_str).right_aligned()).fg(theme.text_focused);

                Row::new([
                    track_no_cell,
                    title_cell.into(),
                    artist_cell,
                    format,
                    duration_cell,
                ])
            })
            .collect::<Vec<Row>>();

        let year_str = album
            .year
            .filter(|y| *y != 0)
            .map_or(String::new(), |y| format!("[{y}]"));

        let title_line = Line::from_iter([
            Span::from(format!(" {} ", album_title))
                .fg(theme.text_secondary)
                .italic(),
            Span::from(year_str).fg(theme.text_faded),
            Span::from(" ✧ ").fg(theme.text_faded),
            Span::from(album.artist.as_str()).fg(theme.text_focused),
            Span::from(format!(" [{} Songs] ", album.tracklist.len())).fg(theme.text_faded),
        ]);

        let header = get_header(&state.get_mode(), &state.get_table_sort());
        let widths = get_widths(&state.get_mode());

        let keymaps = match state.get_pane() {
            Pane::TrackList => " [q] Queue Song ✧ [Tab] Back ".fg(theme.text_faded),
            _ => "".into(),
        };

        let block = Block::bordered()
            .title_top(title_line)
            .title_bottom(keymaps)
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Thick)
            .border_style(Style::default().fg(theme.border))
            .bg(theme.bg)
            .padding(PADDING);

        let table = Table::new(rows, widths)
            .header(
                Row::new(header)
                    .fg(theme.text_secondary)
                    .bottom_margin(1)
                    .bold(),
            )
            .column_spacing(COLUMN_SPACING)
            .flex(Flex::Start)
            .block(block)
            .highlight_symbol(SELECTOR)
            .highlight_spacing(HighlightSpacing::Always)
            .row_highlight_style(
                Style::default()
                    .bg(theme.text_highlighted)
                    .fg(Color::Black)
                    .italic(),
            );

        // RENDER THE TABLE
        StatefulWidget::render(table, area, buf, &mut state.table_pos);
    }
}

fn get_track_discs(
    song: &Arc<SimpleSong>,
    disc_count: usize,
    theme: &DisplayTheme,
) -> Cell<'static> {
    let track_no = Span::from(match song.track_no {
        Some(t) => format!("{t:>2}"),
        None => "".into(),
    })
    .fg(theme.text_highlighted);

    let disc_no = Span::from(match disc_count {
        0..2 => "".to_string(),
        _ => match song.disc_no {
            Some(t) => String::from("ᴰ") + SUPERSCRIPT.get(&t).unwrap_or(&"?"),
            None => "".into(),
        },
    })
    .fg(theme.text_faded);

    Cell::from(Line::from_iter([track_no, " ".into(), disc_no.into()]))
}
