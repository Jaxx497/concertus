mod album_tracklist;
mod generic_tracklist;
mod search_results;

use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
};

pub use album_tracklist::AlbumView;
pub use generic_tracklist::GenericView;
pub use search_results::StandardTable;

use crate::{
    domain::{SimpleSong, SongInfo},
    get_readable_duration,
    ui_state::{DisplayTheme, Mode, Pane, TableSort, UiState},
    DurationStyle,
};
use ratatui::{
    layout::{Alignment, Constraint, Flex},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Cell, HighlightSpacing, Padding, Row, Table},
};

const COLUMN_SPACING: u16 = 2;
const SELECTOR: &str = "⮞  ";

const PADDING: Padding = Padding {
    left: 2,
    right: 3,
    top: 1,
    bottom: 1,
};

pub(super) fn get_widths(mode: &Mode) -> Vec<Constraint> {
    match mode {
        Mode::Power | Mode::Search => {
            vec![
                Constraint::Ratio(3, 9),
                Constraint::Ratio(2, 9),
                Constraint::Ratio(2, 9),
                Constraint::Length(8),
            ]
        }
        Mode::Library(_) | Mode::Queue => {
            vec![
                Constraint::Length(6),
                Constraint::Min(25),
                Constraint::Max(30),
                Constraint::Max(6),
                Constraint::Length(7),
            ]
        }
        _ => Vec::new(),
    }
}

pub(super) fn get_header<'a>(state: &UiState, active: &TableSort) -> Row<'a> {
    let row = match state.get_mode() {
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
        Mode::Library(_) | Mode::Queue => {
            vec![
                Text::default(),
                Text::from("Title").underlined(),
                Text::from("Artist").underlined(),
                Text::from("Format").underlined(),
                Text::from("Length").right_aligned().underlined(),
            ]
        }
        _ => Vec::new(),
    };

    Row::new(row).bottom_margin(1).bold()
}

pub fn get_keymaps(pane: &Pane) -> &'static str {
    match pane {
        &Pane::TrackList => " [q]ueue song ✧ [a]dd to playlist ✧ [x] remove ",
        _ => "",
    }
}

pub fn create_standard_table<'a>(
    rows: Vec<Row<'a>>,
    title: Line<'static>,
    state: &UiState,
) -> Table<'a> {
    let pane = state.get_pane();
    let mode = state.get_mode();
    let theme = state.get_theme(&Pane::TrackList);

    let header = get_header(state, &TableSort::Title);
    let widths = get_widths(mode);
    let keymaps = get_keymaps(pane);

    let block = Block::bordered()
        .title_top(Line::from(title).alignment(Alignment::Center))
        .title_bottom(Line::from(keymaps.fg(theme.text_faded)))
        .title_alignment(Alignment::Center)
        .borders(theme.border_display)
        .border_type(BorderType::Thick)
        .border_style(theme.border)
        .padding(PADDING)
        .bg(theme.bg);

    Table::new(rows, widths)
        .block(block)
        .header(header.fg(theme.text_secondary))
        .column_spacing(COLUMN_SPACING)
        .flex(Flex::Start)
        .highlight_symbol(SELECTOR)
        .highlight_spacing(HighlightSpacing::Always)
        .row_highlight_style(
            Style::new()
                .fg(Color::Black)
                .bg(theme.text_highlighted)
                .italic(),
        )
}

pub struct CellFactory;

impl CellFactory {
    pub fn title_cell(
        theme: &DisplayTheme,
        song: &Arc<SimpleSong>,
        playing: bool,
        queued: bool,
    ) -> Cell<'static> {
        let title = match (queued, playing) {
            (true, false) => Line::from_iter([
                song.get_title().to_string().fg(theme.text_focused),
                " [queued]".fg(theme.text_faded).italic().into(),
            ]),
            (false, true) => Line::from_iter([
                song.get_title().to_string().fg(theme.text_focused),
                " ♫".fg(theme.text_secondary).into(),
            ]),
            _ => Line::from(song.get_title().to_string().fg(theme.text_focused)),
        };
        Cell::from(title)
    }

    pub fn artist_cell(theme: &DisplayTheme, song: &Arc<SimpleSong>) -> Cell<'static> {
        Cell::from(song.get_artist().to_string()).fg(theme.text_focused)
    }

    pub fn filetype_cell(theme: &DisplayTheme, song: &Arc<SimpleSong>) -> Cell<'static> {
        Cell::from(format!("{}", song.filetype)).fg(theme.text_secondary)
    }

    pub fn duration_cell(theme: &DisplayTheme, song: &Arc<SimpleSong>) -> Cell<'static> {
        let duration_str = get_readable_duration(song.get_duration(), DurationStyle::Clean);
        Cell::from(Text::from(duration_str).right_aligned()).fg(theme.text_focused)
    }

    pub fn index_cell(theme: &DisplayTheme, index: usize) -> Cell<'static> {
        Cell::from(format!("{:>2}", index + 1)).fg(theme.text_highlighted)
    }

    pub fn get_track_discs(
        theme: &DisplayTheme,
        song: &Arc<SimpleSong>,
        disc_count: usize,
    ) -> Cell<'static> {
        let track_no = Span::from(match song.track_no {
            Some(t) => format!("{t:>2}"),
            None => format!("{x:>2}", x = "󰇘"),
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
}

static SUPERSCRIPT: LazyLock<HashMap<u32, &str>> = LazyLock::new(|| {
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
