use super::tracklist::{AlbumView, QueueTable, StandardTable};
use crate::{
    tui::widgets::tracklist::PlaylistView,
    ui_state::{LibraryView, Mode, UiState},
};
use ratatui::widgets::StatefulWidget;

pub struct SongTable;
impl StatefulWidget for SongTable {
    type State = UiState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        match state.get_mode() {
            &Mode::Library(LibraryView::Albums) => AlbumView.render(area, buf, state),
            &Mode::Library(LibraryView::Playlists) => PlaylistView.render(area, buf, state),
            &Mode::Queue => QueueTable.render(area, buf, state),
            _ => StandardTable.render(area, buf, state),
        }
    }
}
