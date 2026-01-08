use crate::{
    library::SongInfo,
    tui::widgets::tracklist::{CellFactory, create_standard_table},
    ui_state::{MatchField, Pane, UiState, fade_color},
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
        let focus = matches!(state.get_pane(), Pane::TrackList | Pane::Search);
        let theme = &state.theme_manager.get_display_theme(focus);

        let songs = state.get_legal_songs().to_owned();
        let song_len = songs.len();
        let search_len = state.get_search_len();

        let title = match state.get_mode() {
            _ => match search_len > 1 {
                true => format!(" Search Results: {} Songs ", song_len),
                false => format!(" Total: {} Songs ", song_len),
            },
        };

        let inactive = fade_color(theme.dark, theme.text_primary, 0.6);
        let rows = songs
            .iter()
            .map(|song| {
                let symbol = CellFactory::status_cell(song, state, true);
                let mut title_col = Cell::from(song.get_title()).fg(inactive);
                let mut artist_col = Cell::from(song.get_artist()).fg(inactive);
                let mut album_col = Cell::from(song.get_album()).fg(inactive);
                let dur_col =
                    Cell::from(Line::from(song.get_duration_str()).right_aligned()).fg(inactive);

                if let Some(field) = state.get_match_fields(song.id) {
                    match field {
                        MatchField::Title => title_col = title_col.fg(theme.text_secondary),
                        MatchField::Artist => artist_col = artist_col.fg(theme.text_secondary),
                        MatchField::Album => album_col = album_col.fg(theme.text_secondary),
                    }
                }

                Row::new([symbol, title_col, artist_col, album_col, dur_col])
            })
            .collect::<Vec<Row>>();

        let table = create_standard_table(rows, title.into(), state, theme);

        StatefulWidget::render(table, area, buf, &mut state.display_state.table_pos);
    }
}
