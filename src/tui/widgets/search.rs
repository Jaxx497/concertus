use crate::{
    tui::SEARCH_PADDING,
    ui_state::{Pane, UiState},
};
use ratatui::{
    style::{Style, Stylize},
    widgets::{Block, StatefulWidget, Widget},
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
        search.set_style(Style::new().bg(theme.bg));
        search.set_block(Block::new().padding(SEARCH_PADDING).bg(theme.bg));

        search.render(area, buf);
    }
}
