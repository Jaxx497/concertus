use std::collections::HashSet;

use crate::{
    domain::SongInfo,
    tui::widgets::tracklist::{create_standard_table, CellFactory},
    ui_state::{LibraryView, Mode, Pane, UiState},
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

        let pretitle = match state.get_mode() {
            Mode::Queue => "Queue",
            Mode::Library(LibraryView::Playlists) => "Playlist",
            _ => "",
        };

        let title = format!(" {} Size: {} Songs ", pretitle, songs.len());

        // let now_playing = state.get_now_playing().map(|s| s.id);

        // let queued_ids: HashSet<u64> = state.playback.queue.iter().map(|s| s.get_id()).collect();
        //
        let rows = songs
            .iter()
            .enumerate()
            .map(|(idx, song)| {
                // let playing = now_playing == Some(song.id);
                // let queued = queued_ids.contains(&song.id);

                let index = CellFactory::index_cell(&theme, idx);
                let icon = CellFactory::status_cell(song, state);
                let title = CellFactory::title_cell(&theme, song);
                // let title = CellFactory::title_cell(&theme, song, playing, queued);
                let artist = CellFactory::artist_cell(&theme, song);
                let filetype = CellFactory::filetype_cell(&theme, song);
                let duration = CellFactory::duration_cell(&theme, song);

                Row::new([index, icon, title, artist, filetype, duration])
            })
            .collect::<Vec<Row>>();

        let table = create_standard_table(rows, title.into(), state);
        StatefulWidget::render(table, area, buf, &mut state.display_state.table_pos);
    }
}
