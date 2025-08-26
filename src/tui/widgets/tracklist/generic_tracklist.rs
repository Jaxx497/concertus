use super::{get_widths, COLUMN_SPACING, PADDING};
use crate::{
    domain::SongInfo,
    get_readable_duration,
    tui::widgets::tracklist::{create_standard_table, get_header, get_keymaps},
    ui_state::{LibraryView, Mode, Pane, TableSort, UiState},
    DurationStyle,
};
use ratatui::{
    layout::{Alignment, Flex},
    style::{Color, Style, Stylize},
    text::Text,
    widgets::{StatefulWidget, *},
};

pub struct GenericView;
impl StatefulWidget for GenericView {
    type State = UiState;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let theme = &state.get_theme(&Pane::TrackList);
        let songs = state.legal_songs.as_slice();

        let pretitle = match state.get_mode() {
            Mode::Queue => "Queue",
            Mode::Library(LibraryView::Playlists) => "Playlist",
            _ => "",
        };

        let title = format!(" {} Size: {} Songs ", pretitle, songs.len());

        let rows = songs
            .iter()
            .enumerate()
            .map(|(idx, song)| {
                let index = Cell::from(format!("{:>3}", idx + 1)).fg(theme.text_highlighted);
                let title_col = Cell::from(song.get_title()).fg(theme.text_focused);
                let artist_col = Cell::from(song.get_artist()).fg(theme.text_focused);
                let format_col = Cell::from(song.format.to_string()).fg(theme.text_secondary);
                let duration_str = get_readable_duration(song.duration, DurationStyle::Clean);

                let dur_col =
                    Cell::from(Text::from(duration_str).right_aligned()).fg(theme.text_focused);

                Row::new([index, title_col, artist_col, format_col, dur_col])
            })
            .collect::<Vec<Row>>();

        let table = create_standard_table(rows, title, state);
        StatefulWidget::render(table, area, buf, &mut state.display_state.table_pos);
    }
}
