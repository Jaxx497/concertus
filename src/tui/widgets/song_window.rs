use ratatui::widgets::StatefulWidget;

use crate::ui_state::{Mode, UiState};

use super::tracklist::{AlbumView, QueueTable, StandardTable};

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
            &Mode::Album => AlbumView.render(area, buf, state),
            &Mode::Queue => QueueTable.render(area, buf, state),
            _ => StandardTable.render(area, buf, state),
        }
    }
}
