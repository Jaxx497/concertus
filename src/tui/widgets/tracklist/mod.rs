mod album_tracklist;
mod generic_tracklist;
mod search_results;

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, LazyLock},
};

pub use album_tracklist::AlbumView;
pub use generic_tracklist::GenericView;
pub use search_results::StandardTable;

use crate::{
    domain::{SimpleSong, SongInfo},
    get_readable_duration,
    ui_state::{DisplayTheme, LibraryView, Mode, Pane, TableSort, UiState},
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
                Constraint::Length(1),
                Constraint::Ratio(3, 9),
                Constraint::Ratio(2, 9),
                Constraint::Ratio(2, 9),
                Constraint::Length(8),
            ]
        }
        Mode::Library(_) | Mode::Queue => {
            vec![
                Constraint::Length(6),
                Constraint::Length(1),
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
            String::new(),
            TableSort::Title.to_string(),
            TableSort::Artist.to_string(),
            TableSort::Album.to_string(),
            TableSort::Duration.to_string(),
        ]
        .iter()
        .map(
            |s| match (*s == active.to_string(), s.eq(&String::from("Duration"))) {
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
            },
        )
        .collect(),
        Mode::Library(_) | Mode::Queue => {
            vec![
                Text::default(),
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

pub fn get_keymaps(mode: &Mode) -> &'static str {
    matches!(mode, Mode::Library(LibraryView::Playlists) | Mode::Queue)
        .then_some(" [q]ueue ✧ [a]dd to playlist ✧ [x] remove ")
        .unwrap_or(" [q]ueue ✧ [a]dd to playlist ")
}

pub fn create_standard_table<'a>(
    rows: Vec<Row<'a>>,
    title: Line<'static>,
    state: &UiState,
) -> Table<'a> {
    let mode = state.get_mode();
    let theme = state.get_theme(&Pane::TrackList);

    let header = get_header(state, &TableSort::Title);
    let widths = get_widths(mode);
    let keymaps = match state.get_pane() {
        Pane::TrackList => get_keymaps(mode),
        _ => "",
    };

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
    pub fn status_cell(song: &Arc<SimpleSong>, state: &UiState) -> Cell<'static> {
        let theme = state.get_theme(&Pane::TrackList);

        let is_playing = state.get_now_playing().map(|s| s.id) == Some(song.id);

        let is_queued = state
            .playback
            .queue
            .iter()
            .map(|s| s.get_id())
            .collect::<HashSet<_>>()
            .contains(&song.id);
        let is_bulk_selected = state.get_bulk_sel().contains(song);

        Cell::from(if is_playing {
            "♫".fg(theme.text_secondary)
        } else if is_bulk_selected {
            "󱕣".fg(theme.text_highlighted)
        } else if is_queued {
            "".fg(theme.text_highlighted)
        } else {
            "".into()
        })
    }

    pub fn title_cell(theme: &DisplayTheme, song: &Arc<SimpleSong>) -> Cell<'static> {
        Cell::from(song.get_title().to_string().fg(theme.text_focused))
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
