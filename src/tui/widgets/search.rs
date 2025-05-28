use crate::{
    tui::SEARCH_PADDING,
    ui_state::{Pane, UiState},
};
use ratatui::{
    style::Stylize,
    widgets::{Block, BorderType, StatefulWidget, Widget},
};

pub struct SearchBar;

impl StatefulWidget for SearchBar {
    type State = UiState;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let theme = &state.get_theme(&Pane::Search);
        let search = state.get_search_widget();
        search.set_block(
            Block::bordered()
                .border_type(BorderType::Thick)
                .padding(SEARCH_PADDING)
                .fg(theme.text_highlighted),
        );

        search.render(area, buf);
    }
}
