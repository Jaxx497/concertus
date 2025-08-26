use super::{SideBarAlbum, SideBarPlaylist};
use crate::ui_state::{LibraryView, UiState};
use ratatui::widgets::StatefulWidget;

pub struct SideBarHandler;
impl StatefulWidget for SideBarHandler {
    type State = UiState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        match state.get_sidebar_view() {
            LibraryView::Albums => SideBarAlbum.render(area, buf, state),
            LibraryView::Playlists => SideBarPlaylist.render(area, buf, state),
        }
    }
}
