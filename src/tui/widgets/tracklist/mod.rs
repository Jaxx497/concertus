mod album_tracklist;
mod generic_tracklist;
mod search_results;

pub use album_tracklist::AlbumView;
pub use generic_tracklist::GenericView;
pub use search_results::StandardTable;

use crate::ui_state::{LibraryView, Mode, Pane, TableSort, UiState};
use ratatui::{
    layout::{Alignment, Constraint, Flex},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, HighlightSpacing, Padding, Row, Table},
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
        Mode::Library(LibraryView::Albums) => {
            vec![
                Constraint::Length(6),
                Constraint::Min(25),
                Constraint::Max(30),
                Constraint::Max(6),
                Constraint::Length(7),
            ]
        }
        Mode::Library(LibraryView::Playlists) | Mode::Queue => {
            vec![
                Constraint::Min(6),
                Constraint::Min(30),
                Constraint::Fill(30),
                Constraint::Max(5),
                Constraint::Max(6),
            ]
        }
        _ => Vec::new(),
    }
}

pub(super) fn get_header<'a>(mode: &Mode, active: &TableSort) -> Vec<Text<'a>> {
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
    }
}

pub fn get_keymaps(pane: &Pane) -> &'static str {
    match pane {
        &Pane::TrackList => " [q]ueue song ✧ [a]dd to playlist ✧ [x] remove ",
        _ => "",
    }
}

pub fn create_standard_table<'a>(rows: Vec<Row<'a>>, title: String, state: &UiState) -> Table<'a> {
    let pane = state.get_pane();
    let mode = state.get_mode();
    let theme = state.get_theme(&Pane::TrackList);

    let header = get_header(mode, &TableSort::Title);
    let widths = get_widths(mode);
    let keymaps = get_keymaps(pane);

    let block = Block::bordered()
        .title_top(Line::from(title).alignment(Alignment::Center))
        .title_bottom(Line::from(keymaps.fg(theme.text_faded)).alignment(Alignment::Center))
        .borders(theme.border_display)
        .border_type(BorderType::Thick)
        .border_style(theme.border)
        .bg(theme.bg)
        .padding(PADDING);

    Table::new(rows, widths)
        .block(block)
        .header(
            Row::new(header)
                .fg(theme.text_secondary)
                .bottom_margin(1)
                .bold(),
        )
        .column_spacing(COLUMN_SPACING)
        .flex(Flex::SpaceBetween)
        .highlight_symbol(SELECTOR)
        .highlight_spacing(HighlightSpacing::Always)
        .row_highlight_style(
            Style::new()
                .fg(Color::Black)
                .bg(theme.text_highlighted)
                .italic(),
        )
}
