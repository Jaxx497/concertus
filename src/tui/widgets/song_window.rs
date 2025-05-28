use crate::{
    domain::{SimpleSong, SongInfo},
    get_readable_duration, truncate_at_last_space,
    ui_state::{DisplayTheme, Mode, Pane, TableSort, UiState, GOOD_RED},
    DurationStyle,
};
use ratatui::{
    layout::{Alignment, Constraint, Flex},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{StatefulWidget, *},
};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, LazyLock},
};

use super::SELECTOR;

static PADDING: Padding = Padding {
    left: 2,
    right: 3,
    top: 1,
    bottom: 1,
};

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

pub struct SongTable;
impl StatefulWidget for SongTable {
    type State = UiState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        match state.get_mode() {
            &Mode::Album => AlbumView.render(area, buf, state),
            &Mode::Queue => QueueTable.render(area, buf, state),
            _ => StandardTable.render(area, buf, state),
        }
    }
}

pub struct StandardTable;
impl StatefulWidget for StandardTable {
    type State = UiState;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let theme = &state.get_theme(&Pane::TrackList);

        let songs = state.legal_songs.as_slice();
        let song_len = songs.len();
        let search_len = state.get_search_len();

        let results = match state.get_mode() {
            Mode::Queue => format!(" Queue Size: {} Songs ", song_len),
            _ => match search_len > 1 {
                true => format!(" Search Results: {} Songs ", song_len),
                false => format!(" Total: {} Songs ", song_len),
            },
        };

        let rows = songs
            .iter()
            .map(|song| {
                let mut title_col = Cell::from(song.get_title()).fg(theme.text_faded);
                let mut artist_col = Cell::from(song.get_artist()).fg(theme.text_faded);
                let mut album_col = Cell::from(song.get_album()).fg(theme.text_faded);
                let mut dur_col = Cell::from(Line::from(song.get_duration_str()).right_aligned())
                    .fg(theme.text_faded);

                match state.get_table_sort() {
                    TableSort::Title => title_col = title_col.fg(theme.text_focused),
                    TableSort::Album => album_col = album_col.fg(theme.text_focused),
                    TableSort::Artist => artist_col = artist_col.fg(theme.text_focused),
                    TableSort::Duration => dur_col = dur_col.fg(theme.text_focused),
                }
                Row::new([title_col, artist_col, album_col, dur_col])
            })
            .collect::<Vec<Row>>();

        let header = Row::new(get_header(&state.get_mode(), &state.get_table_sort()))
            .bold()
            .fg(theme.text_secondary)
            .bottom_margin(1);
        let widths = get_widths(&state.get_mode());

        let table = Table::new(rows, widths)
            .column_spacing(5)
            .header(header)
            .flex(Flex::Legacy)
            .block(
                Block::new()
                    .title_top(Line::from(results).alignment(Alignment::Center))
                    .borders(Borders::NONE)
                    .border_type(BorderType::default())
                    .padding(PADDING)
                    .fg(theme.text_focused)
                    .bg(theme.bg),
            )
            .row_highlight_style(Style::new().fg(theme.text_highlighted).italic())
            .highlight_spacing(HighlightSpacing::Always)
            .highlight_symbol(SELECTOR);

        StatefulWidget::render(table, area, buf, &mut state.table_pos);
    }
}

pub struct QueueTable;
impl StatefulWidget for QueueTable {
    type State = UiState;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let theme = &state.get_theme(&Pane::TrackList);

        let songs = state.legal_songs.as_slice();
        let song_len = songs.len();

        let results = format!(" Queue Size: {} Songs ", song_len);

        let rows = songs
            .iter()
            .enumerate()
            .map(|(idx, song)| {
                let index = Cell::from(format!("{:>2}", idx + 1)).fg(theme.text_faded);
                let title_col = Cell::from(song.get_title()).fg(theme.text_focused);
                let artist_col = Cell::from(song.get_artist()).fg(theme.text_focused);
                let format_col = Cell::from(song.format.to_string()).fg(theme.text_secondary);
                let duration_str = get_readable_duration(song.duration, DurationStyle::Clean);

                let dur_col =
                    Cell::from(Text::from(duration_str).right_aligned()).fg(theme.text_focused);

                Row::new([index, title_col, artist_col, format_col, dur_col])
            })
            .collect::<Vec<Row>>();

        let widths = get_widths(&state.get_mode());

        let table = Table::new(rows, widths)
            .column_spacing(10)
            .flex(Flex::Legacy)
            .block(
                Block::new()
                    .title_top(Line::from(results).alignment(Alignment::Center))
                    .borders(Borders::NONE)
                    .border_type(BorderType::default())
                    .padding(PADDING)
                    .fg(theme.text_focused)
                    .bg(theme.bg),
            )
            .highlight_spacing(HighlightSpacing::Always)
            .row_highlight_style(theme.text_highlighted)
            .highlight_symbol(SELECTOR);

        StatefulWidget::render(table, area, buf, &mut state.table_pos);
    }
}

pub struct AlbumView;
impl StatefulWidget for AlbumView {
    type State = UiState;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let theme = &state.get_theme(&Pane::TrackList);

        let album_list = &state.filtered_albums;
        if album_list.is_empty() {
            return;
        }
        let album_idx = state.album_pos.selected().unwrap_or(0);
        let album = &album_list[album_idx];

        let album_title_raw = state.get_selected_album_title();
        let album_title_width = (area.width / 3) as usize;
        let album_title = truncate_at_last_space(album_title_raw, album_title_width);

        let queued_songs = state
            .queue
            .iter()
            .map(|s| s.get_id())
            .collect::<HashSet<u64>>();

        let now_playing = state.get_now_playing().map(|s| s.id);

        let year = album.year.unwrap_or(0);

        let year_str = match year == 0 {
            true => String::new(),
            false => format!(" [{year}]"),
        };

        let artist = album.artist.as_str();

        let songs = &state.legal_songs;
        let song_len = songs.len();

        let disc_count = songs
            .iter()
            .filter_map(|s| s.disc_no)
            .collect::<HashSet<_>>()
            .len();

        let rows = songs
            .iter()
            .map(|song| {
                let title_cell = match (
                    queued_songs.get(&song.id).is_some(),
                    now_playing == Some(song.id),
                ) {
                    (true, false) => Cell::from(Line::from_iter([
                        song.get_title().fg(theme.text_focused),
                        " [queued]".fg(theme.text_faded).italic().into(),
                    ])),
                    (false, true) => Cell::from(Line::from_iter([
                        song.get_title().fg(theme.text_focused),
                        " ♫".fg(GOOD_RED).into(),
                    ])),
                    _ => Cell::from(Line::from_iter([song.get_title().fg(theme.text_focused)])),
                };

                let track_no_cell = get_track_discs(song, disc_count, theme);
                let artist_cell = Cell::from(song.get_artist()).fg(theme.text_focused);
                let format = Cell::from(format!("[{}]", song.format)).fg(theme.text_secondary);

                let duration_str = get_readable_duration(song.duration, DurationStyle::Clean);

                let duration_cell =
                    Cell::from(Text::from(duration_str).right_aligned()).fg(theme.text_focused);

                Row::new([
                    track_no_cell,
                    title_cell,
                    artist_cell,
                    format,
                    duration_cell,
                ])
            })
            .collect::<Vec<Row>>();

        let header = get_header(&state.get_mode(), &state.get_table_sort());
        let widths = get_widths(&state.get_mode());

        let title_line = Line::from_iter([
            " ".into(),
            Span::from(album_title).fg(theme.text_secondary).italic(),
            Span::from(year_str).fg(theme.text_faded),
            Span::from(" ✧ ").fg(theme.text_faded),
            Span::from(artist).fg(theme.text_focused),
            Span::from(" [").fg(theme.text_faded),
            Span::from(song_len.to_string()).fg(theme.text_faded),
            Span::from(" Songs] ").fg(theme.text_faded),
        ]);

        let block = Block::bordered()
            .title_top(title_line)
            .title_bottom(" [q] Queue Song • [Tab] Back ".fg(theme.text_faded))
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Thick)
            .border_style(Style::default().fg(theme.text_secondary))
            .bg(theme.bg)
            .padding(PADDING);

        let table = Table::new(rows, widths)
            .header(
                Row::new(header)
                    .fg(theme.text_secondary)
                    .bottom_margin(1)
                    .bold(),
            )
            .column_spacing(3)
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

fn get_widths(mode: &Mode) -> Vec<Constraint> {
    match mode {
        Mode::Power | Mode::Search => {
            vec![
                Constraint::Ratio(3, 9),
                Constraint::Ratio(2, 9),
                Constraint::Ratio(2, 9),
                Constraint::Length(8),
            ]
        }
        Mode::Album => {
            vec![
                Constraint::Length(6),
                Constraint::Max(40),
                Constraint::Max(30),
                Constraint::Fill(6),
                Constraint::Max(8),
            ]
        }
        Mode::Queue => {
            vec![
                Constraint::Length(3),
                Constraint::Min(25),
                Constraint::Min(10),
                Constraint::Max(5),
                Constraint::Max(6),
            ]
        }
        _ => Vec::new(),
    }
}

fn get_header<'a>(mode: &Mode, active: &TableSort) -> Vec<Text<'a>> {
    match mode {
        Mode::Power | Mode::Search => [
            TableSort::Title,
            TableSort::Artist,
            TableSort::Album,
            TableSort::Duration,
        ]
        .iter()
        .map(|s| match (s == active, s.eq(&TableSort::Duration)) {
            (true, true) => Text::from(s.to_string())
                .fg(Color::Red)
                .underlined()
                .italic()
                .right_aligned(),
            (false, true) => Text::from(s.to_string()).right_aligned(),
            (true, false) => Text::from(Span::from(
                s.to_string().fg(Color::Red).underlined().italic(),
            )),
            _ => s.to_string().into(),
        })
        .collect(),
        Mode::Album => {
            vec![
                Text::default(),
                Text::from("Title").underlined(),
                Text::from("Artist").underlined(),
                Text::from("Format").underlined(),
                Text::from("Duration").right_aligned().underlined(),
            ]
        }
        _ => Vec::new(),
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
    .fg(theme.text_focused);

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
