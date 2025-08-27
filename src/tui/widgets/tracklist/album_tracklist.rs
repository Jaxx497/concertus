use crate::{
    domain::SongInfo,
    truncate_at_last_space,
    tui::widgets::tracklist::{create_standard_table, CellFactory},
    ui_state::{Pane, UiState},
};
use ratatui::{
    style::Stylize,
    text::{Line, Span},
    widgets::{Row, StatefulWidget},
};
use std::collections::HashSet;

pub struct AlbumView;
impl StatefulWidget for AlbumView {
    type State = UiState;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        if state.albums.is_empty() {
            return;
        }

        let theme = &state.get_theme(&Pane::TrackList);

        let album = state
            .get_selected_album()
            .unwrap_or(&state.albums[0])
            .clone();

        let album_title = truncate_at_last_space(&album.title, (area.width / 3) as usize);

        let queued_ids: HashSet<u64> = state.playback.queue.iter().map(|s| s.get_id()).collect();
        let now_playing_id = state.get_now_playing().map(|s| s.id);

        let disc_count = album
            .tracklist
            .iter()
            .filter_map(|s| s.disc_no)
            .max()
            .unwrap_or(1) as usize;

        let rows = album
            .tracklist
            .iter()
            .map(|song| {
                let is_queued = queued_ids.contains(&song.id);
                let is_playing = now_playing_id == Some(song.id);

                let track_no = CellFactory::get_track_discs(theme, song, disc_count);
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
            Span::from(album.artist.to_string()).fg(theme.text_focused),
            Span::from(format!(" [{} Songs] ", album.tracklist.len())).fg(theme.text_faded),
        ]);

        let table = create_standard_table(rows, title, state);
        StatefulWidget::render(table, area, buf, &mut state.display_state.table_pos);
    }
}
