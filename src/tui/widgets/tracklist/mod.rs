mod album_view;
mod playlist_view;
mod queue_view;
mod search_results;

pub use album_view::AlbumView;
pub use playlist_view::PlaylistView;
pub use queue_view::QueueTable;
pub use search_results::StandardTable;

use crate::ui_state::{LibraryView, Mode, Pane, TableSort};
use ratatui::{
    layout::Constraint,
    style::{Color, Stylize},
    text::{Span, Text},
    widgets::Padding,
};

const COLUMN_SPACING: u16 = 2;
const SELECTOR: &str = "⮞  ";
const keymaps: &str = " [q]ueue song | [a]dd to playlist ";

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
                Constraint::Min(3),
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
        Mode::Library(LibraryView::Albums) => {
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
        &Pane::TrackList => " [q]ueue Song ✧ [a]dd to Playlist ✧ [Tab] Back ",
        _ => "",
    }
}
