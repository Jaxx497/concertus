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
    DurationStyle,
    domain::{SimpleSong, SongInfo},
    get_readable_duration,
    tui::widgets::{DECORATOR, MUSIC_NOTE, QUEUED},
    ui_state::{DisplayTheme, LibraryView, Mode, Pane, TableSort, UiState},
};
use ratatui::{
    layout::{Alignment, Constraint, Flex, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Cell, Padding, Row, Table},
};

const COLUMN_SPACING: u16 = 2;

const PADDING: Padding = Padding {
    left: 4,
    right: 4,
    top: 2,
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
                Constraint::Max(20),
                Constraint::Max(4),
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
                Text::from("𝕋𝕚𝕥𝕝𝕖").bold(),
                Text::from("𝔸𝕣𝕥𝕚𝕤𝕥"),
                Text::from("").centered(),
                Text::from("").centered(),
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

    let widths = get_widths(mode);
    let keymaps = match state.get_pane() {
        Pane::TrackList => get_keymaps(mode),
        _ => "",
    };

    let block = Block::bordered()
        .borders(theme.border_display)
        .border_type(theme.border_type)
        .border_style(theme.border)
        .title_top(Line::from(title).alignment(Alignment::Center))
        .title_bottom(Line::from(keymaps.fg(theme.text_muted)))
        .title_alignment(Alignment::Center)
        .padding(PADDING)
        .bg(theme.bg);

    let highlight_style = match state.get_pane() {
        Pane::TrackList => Style::new().fg(theme.text_selected).bg(theme.selection),
        _ => Style::new(),
    };

    Table::new(rows, widths)
        .block(block)
        .column_spacing(COLUMN_SPACING)
        .flex(Flex::Start)
        .row_highlight_style(highlight_style)
}

pub fn create_empty_block(theme: &DisplayTheme, title: &str) -> Block<'static> {
    Block::bordered()
        .borders(theme.border_display)
        .border_type(theme.border_type)
        .border_style(theme.border)
        .title_top(format!(" {} ", title))
        .title_alignment(Alignment::Center)
        .padding(PADDING)
        .bg(theme.bg)
}

pub struct CellFactory;

impl CellFactory {
    pub fn status_cell(song: &Arc<SimpleSong>, state: &UiState, ms: bool) -> Cell<'static> {
        let theme = state.get_theme(&Pane::TrackList);

        let is_playing = state.get_now_playing().map(|s| s.id) == Some(song.id);
        let is_queued = state.playback.queue_ids.contains(&song.id);

        Cell::from(if is_playing {
            MUSIC_NOTE.fg(match ms {
                true => theme.text_selected,
                false => theme.text_secondary,
            })
        } else if is_queued && !matches!(state.get_mode(), Mode::Queue) {
            QUEUED.fg(match ms {
                true => theme.text_selected,
                false => theme.text_secondary,
            })
        } else {
            "".into()
        })
    }

    pub fn title_cell(theme: &DisplayTheme, title: &str, ms: bool) -> Cell<'static> {
        Cell::from(title.to_owned()).fg(match ms {
            true => theme.text_selected,
            false => theme.text_primary,
        })
    }

    pub fn artist_cell(theme: &DisplayTheme, song: &Arc<SimpleSong>, ms: bool) -> Cell<'static> {
        Cell::from(Line::from(song.get_artist().to_string())).fg(set_color_selection(ms, theme))
    }

    pub fn filetype_cell(theme: &DisplayTheme, song: &Arc<SimpleSong>, ms: bool) -> Cell<'static> {
        Cell::from(Line::from(format!("{}", song.filetype)).centered()).fg(match ms {
            true => theme.text_selected,
            false => theme.text_secondary,
        })
    }

    pub fn duration_cell(theme: &DisplayTheme, song: &Arc<SimpleSong>, ms: bool) -> Cell<'static> {
        let duration_str = get_readable_duration(song.get_duration(), DurationStyle::Clean);
        Cell::from(Text::from(duration_str).right_aligned()).fg(match ms {
            true => theme.text_selected,
            false => theme.text_muted,
        })
    }

    pub fn index_cell(theme: &DisplayTheme, index: usize, ms: bool) -> Cell<'static> {
        Cell::from(format!("{:>2}", index + 1)).fg(set_color_selection(ms, theme))
    }

    pub fn get_track_discs(
        theme: &DisplayTheme,
        song: &Arc<SimpleSong>,
        ms: bool,
    ) -> Cell<'static> {
        let track_no = Span::from(match song.track_no {
            Some(t) => format!("{t:>2}"),
            None => format!("{x:>2}", x = "󰇘"),
        })
        .fg(match ms {
            true => theme.text_selected,
            false => theme.accent,
        });

        let disc_no = Span::from(match song.disc_no {
            Some(t) => String::from("ᴰ") + SUPERSCRIPT.get(&t).unwrap_or(&"?"),
            None => "".into(),
        })
        .fg(match ms {
            true => theme.text_selected,
            false => theme.text_muted,
        });

        Cell::from(Line::from_iter([track_no, " ".into(), disc_no.into()]))
    }
}

fn set_color_selection(selected: bool, theme: &DisplayTheme) -> Color {
    match selected {
        true => theme.text_selected,
        false => theme.text_primary,
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

fn get_title(state: &UiState, area: Rect) -> Line<'static> {
    let theme = state.get_theme(&Pane::TrackList);
    let (title, track_count) = match state.get_mode() {
        &Mode::Queue => (
            Span::from("Queue").fg(theme.accent),
            state.playback.queue.len(),
        ),
        &Mode::Library(LibraryView::Playlists) => {
            if state.playlists.is_empty() {
                return "".into();
            }

            let playlist = match state.get_selected_playlist() {
                Some(p) => p,
                None => return "".into(),
            };

            let formatted_title =
                crate::truncate_at_last_space(&playlist.name, (area.width / 3) as usize);
            (
                Span::from(format!("{}", formatted_title))
                    .fg(theme.text_secondary)
                    .italic(),
                playlist.tracklist.len(),
            )
        }
        _ => (Span::default(), 0),
    };

    Line::from_iter([
        Span::from(DECORATOR).fg(theme.text_primary),
        title,
        Span::from(DECORATOR).fg(theme.text_primary),
        Span::from(format!("[{} Songs] ", track_count)).fg(theme.text_muted),
    ])
}
