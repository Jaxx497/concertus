use crate::{
    tui::widgets::tracklist::{create_standard_table, get_title, CellFactory},
    ui_state::{Pane, UiState},
};
use ratatui::widgets::{Row, StatefulWidget};

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

        let rows = songs
            .iter()
            .enumerate()
            .map(|(idx, song)| {
                let index = CellFactory::index_cell(&theme, idx);
                let icon = CellFactory::status_cell(song, state);
                let title = CellFactory::title_cell(&theme, song);
                let artist = CellFactory::artist_cell(&theme, song);
                let filetype = CellFactory::filetype_cell(&theme, song);
                let duration = CellFactory::duration_cell(&theme, song);

                Row::new([index, icon, title, artist, filetype, duration])
            })
            .collect::<Vec<Row>>();

        let title = get_title(state, area);

        let table = create_standard_table(rows, title.into(), state);
        StatefulWidget::render(table, area, buf, &mut state.display_state.table_pos);
    }
}
