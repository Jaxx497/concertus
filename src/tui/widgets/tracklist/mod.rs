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
    get_readable_duration, truncate_at_last_space,
    tui::widgets::{MUSIC_NOTE, QUEUED},
    ui_state::{DisplayTheme, LibraryView, Mode, Pane, UiState},
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
                Constraint::Length(4),
                Constraint::Length(7),
            ]
        }
        _ => Vec::new(),
    }
}

pub fn get_keymaps(mode: &Mode, decorator: &str) -> String {
    let full = format!(" [q]ueue {decorator} [a]dd to playlist {decorator} [x] remove ");
    let basic = format!(" [q]ueue {decorator} [a]dd to playlist ");

    matches!(mode, Mode::Library(LibraryView::Playlists) | Mode::Queue)
        .then_some(full)
        .unwrap_or(basic)
}

pub fn create_standard_table<'a>(
    rows: Vec<Row<'a>>,
    title: Line<'static>,
    state: &UiState,
    theme: &DisplayTheme,
) -> Table<'a> {
    let mode = state.get_mode();
    let pane = state.get_pane();
    let decorator = &state.get_decorator();

    let widths = get_widths(mode);
    let keymaps = match pane {
        Pane::TrackList => get_keymaps(mode, &decorator),
        _ => String::default(),
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
        .flex(Flex::SpaceBetween)
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
        let focus = matches!(state.get_pane(), Pane::TrackList);
        let theme = state.theme_manager.get_display_theme(focus);

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
            None => format!("{x:>2}", x = ""),
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
    let focus = matches!(state.get_pane(), Pane::TrackList);
    let theme = state.theme_manager.get_display_theme(focus);
    match state.get_mode() {
        &Mode::Queue => {
            let q = state.playback.queue.len();
            let queue_len = match q {
                0 => format!("[0 Songs] "),
                1 => format!("[1 Song] "),
                _ => format!("[{q} Songs] "),
            };

            Line::from_iter([
                Span::from(" Queue ").fg(theme.accent),
                queue_len.fg(theme.text_muted),
            ])
        }
        &Mode::Library(LibraryView::Playlists) => {
            if state.playlists.is_empty() {
                return "".into();
            }

            let playlist = match state.get_selected_playlist() {
                Some(p) => p,
                None => return "".into(),
            };

            let p = playlist.len();
            let playlist_len = match p {
                0 => String::default(),
                1 => format!("1 Song"),
                _ => format!("{p} Songs"),
            };

            let total_length = playlist.get_total_length();
            let readable = get_readable_duration(total_length, DurationStyle::Clean);

            let info = match p {
                0 => String::new(),
                _ => format!("[{playlist_len} ⫽ {readable}] "),
            };

            let truncated_title = truncate_at_last_space(&playlist.name, (area.width / 3) as usize);
            let formatted_title = format!(" {} ", truncated_title);

            Line::from_iter([
                Span::from(formatted_title).fg(theme.text_secondary),
                info.fg(theme.text_muted),
            ])
        }
        _ => Line::default(),
    }
}
