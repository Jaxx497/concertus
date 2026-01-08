use crate::{
    library::SongInfo,
    tui::widgets::tracklist::{CellFactory, create_standard_table, get_title},
    ui_state::{Pane, UiState},
};
use ratatui::{
    style::Stylize,
    widgets::{Row, StatefulWidget},
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
        let focus = matches!(state.get_pane(), Pane::TrackList);
        let theme = &state.theme_manager.get_display_theme(focus);
        let songs = state.get_legal_songs();

        let rows = songs
            .iter()
            .enumerate()
            .map(|(idx, song)| {
                let is_multi_selected = state.get_multi_select_indices().contains(&idx);

                let index = CellFactory::index_cell(&theme, idx, is_multi_selected);
                let icon = CellFactory::status_cell(song, state, is_multi_selected);
                let title = CellFactory::title_cell(&theme, song.get_title(), is_multi_selected);
                let artist = CellFactory::artist_cell(&theme, song, is_multi_selected);
                let filetype = CellFactory::filetype_cell(&theme, song, is_multi_selected);
                let duration = CellFactory::duration_cell(&theme, song, is_multi_selected);

                match is_multi_selected {
                    true => Row::new([index, icon, title, artist, filetype, duration])
                        .fg(theme.text_selected)
                        .bg(state.theme_manager.active.selection_inactive),
                    false => Row::new([index, icon, title, artist, filetype, duration]),
                }
            })
            .collect::<Vec<Row>>();

        let title = get_title(state, area);

        let table = create_standard_table(rows, title, state, theme);
        StatefulWidget::render(table, area, buf, &mut state.display_state.table_pos);
    }
}
