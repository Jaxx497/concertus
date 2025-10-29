use crate::ui_state::{Pane, UiState};
use ratatui::{
    style::Stylize,
    widgets::{Block, StatefulWidget, Widget},
};

const SEARCH_PADDING: ratatui::widgets::Padding = ratatui::widgets::Padding {
    left: 2,
    right: 2,
    top: 1,
    bottom: 0,
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
                .borders(theme.border_display)
                .border_type(theme.border_type)
                .border_style(theme.border)
                .padding(SEARCH_PADDING)
                .fg(theme.accent)
                .bg(theme.bg),
        );

        search.render(area, buf);
    }
}
