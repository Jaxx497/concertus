use crate::{
    library::SongInfo,
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
        let focus = matches!(state.get_pane(), Pane::TrackList);
        let theme = &state.theme_manager.get_display_theme(focus);

        if state.albums.is_empty() {
            create_empty_block(theme, "0 Songs").render(area, buf);
            return;
        }

        let album = state.get_selected_album().unwrap_or(&state.albums[0]);
        let album_title = match &album.title.is_empty() {
            true => String::from("[Unknown Album]"),
            false => truncate_at_last_space(&album.title, (area.width / 3) as usize),
        };

        let rows = album
            .tracklist
            .iter()
            .enumerate()
            .map(|(idx, song)| {
                let is_m_selected = state.get_multi_select_indices().contains(&idx);

                let track_no = CellFactory::get_track_discs(theme, song, is_m_selected);
                let icon = CellFactory::status_cell(song, state, is_m_selected);
                let title = CellFactory::title_cell(theme, song.get_title(), is_m_selected);
                let artist = CellFactory::artist_cell(theme, song, is_m_selected);
                let format = CellFactory::filetype_cell(theme, song, is_m_selected);
                let duration = CellFactory::duration_cell(theme, song, is_m_selected);

                match is_m_selected {
                    true => Row::new([track_no, icon, title.into(), artist, format, duration])
                        .bg(state.theme_manager.active.selection_inactive),
                    false => Row::new([track_no, icon, title.into(), artist, format, duration]),
                }
            })
            .collect::<Vec<Row>>();

        let decorator = &state.get_decorator();

        let year_str = album
            .year
            .filter(|y| *y != 0)
            .map_or(String::new(), |y| format!(" [{y}]"));

        let title = Line::from_iter([
            Span::from(format!(" {}", album_title))
                .fg(theme.text_secondary)
                .italic(),
            Span::from(year_str).fg(theme.text_muted),
            Span::from(format!(" {decorator} ")).fg(theme.text_muted),
            Span::from(album.artist.to_string()).fg(theme.accent),
            Span::from(format!(" [{} Songs] ", album.tracklist.len())).fg(theme.text_muted),
        ]);

        let table = create_standard_table(rows, title, state, theme);
        StatefulWidget::render(table, area, buf, &mut state.display_state.table_pos);
    }
}
