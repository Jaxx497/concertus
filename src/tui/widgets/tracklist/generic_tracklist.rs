use crate::{
    truncate_at_last_space,
    tui::widgets::tracklist::{create_standard_table, CellFactory},
    ui_state::{LibraryView, Mode, Pane, UiState},
};
use ratatui::{
    style::Stylize,
    text::{Line, Span},
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
        let theme = &state.get_theme(&Pane::TrackList);
        let songs = state.legal_songs.as_slice();

        let (title, track_count) = match state.get_mode() {
            &Mode::Queue => (
                Span::from("Queue").fg(theme.text_highlighted),
                state.playback.queue.len(),
            ),
            &Mode::Library(LibraryView::Playlists) => {
                let playlist = state.get_selected_playlist().unwrap_or(&state.playlists[0]);
                let formatted_title =
                    truncate_at_last_space(&playlist.name, (area.width / 3) as usize);

                (
                    Span::from(format!("{}", formatted_title))
                        .fg(theme.text_secondary)
                        .italic(),
                    playlist.tracklist.len(),
                )
            }
            _ => (Span::default(), 0),
        };

        let title = Line::from_iter([
            Span::from(" ♠ ").fg(theme.text_focused),
            title,
            Span::from(" ♠ ").fg(theme.text_focused),
            Span::from(format!("[{} Songs] ", track_count)).fg(theme.text_faded),
        ]);

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

        let table = create_standard_table(rows, title.into(), state);
        StatefulWidget::render(table, area, buf, &mut state.display_state.table_pos);
    }
}
