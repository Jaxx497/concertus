use crate::{
    truncate_at_last_space,
    tui::widgets::tracklist::{CellFactory, create_empty_block, create_standard_table},
    ui_state::{Pane, UiState},
};
use ratatui::{
    style::Stylize,
    text::{Line, Span},
    widgets::{Row, StatefulWidget, Widget},
};

pub struct AlbumView;
impl StatefulWidget for AlbumView {
    type State = UiState;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let theme = &state.get_theme(&Pane::TrackList);

        if state.albums.is_empty() {
            create_empty_block(theme, "0 Songs").render(area, buf);
            return;
        }

        let album = state.get_selected_album().unwrap_or(&state.albums[0]);
        let album_title = truncate_at_last_space(&album.title, (area.width / 3) as usize);

        let rows = album
            .tracklist
            .iter()
            .map(|song| {
                let track_no = CellFactory::get_track_discs(theme, song);
                let icon = CellFactory::status_cell(song, state);
                let title = CellFactory::title_cell(theme, song);
                let artist = CellFactory::artist_cell(theme, song);
                let format = CellFactory::filetype_cell(theme, song);
                let duration = CellFactory::duration_cell(theme, song);

                Row::new([track_no, icon, title.into(), artist, format, duration])
            })
            .collect::<Vec<Row>>();

        let year_str = album
            .year
            .filter(|y| *y != 0)
            .map_or(String::new(), |y| format!("[{y}]"));

        let title = Line::from_iter([
            Span::from(format!(" {} ", album_title))
                .fg(theme.text_secondary)
                .italic(),
            Span::from(year_str).fg(theme.text_faded),
            Span::from(" ✧ ").fg(theme.text_faded),
            Span::from(album.artist.to_string()).fg(theme.text_highlighted),
            Span::from(format!(" [{} Songs] ", album.tracklist.len())).fg(theme.text_faded),
        ]);

        let table = create_standard_table(rows, title, state);
        StatefulWidget::render(table, area, buf, &mut state.display_state.table_pos);
    }
}
