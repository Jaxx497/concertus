use crate::{
    domain::SongInfo,
    tui::widgets::tracklist::create_standard_table,
    ui_state::{Pane, TableSort, UiState},
};
use ratatui::{
    style::Stylize,
    text::Line,
    widgets::{StatefulWidget, *},
};

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

        let title = match state.get_mode() {
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

        let table = create_standard_table(rows, title.into(), state);

        StatefulWidget::render(table, area, buf, &mut state.display_state.table_pos);
    }
}
