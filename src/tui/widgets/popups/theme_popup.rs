use ratatui::{
    layout::Alignment,
    style::Stylize,
    widgets::{Block, List, StatefulWidget},
};

use crate::{
    tui::widgets::{POPUP_PADDING, SELECTOR},
    ui_state::UiState,
};

pub struct ThemeManager;
impl StatefulWidget for ThemeManager {
    type State = UiState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let theme = &state.theme_manager.get_display_theme(true);

        let theme_names = state
            .theme_manager
            .theme_lib
            .iter()
            .map(|t| t.name.clone())
            .collect::<Vec<String>>();

        let block = Block::bordered()
            .border_type(theme.border_type)
            .border_style(theme.border)
            .title(" Select Theme ")
            .title_bottom(" [Enter] / [Esc] ")
            .title_alignment(Alignment::Center)
            .padding(POPUP_PADDING)
            .bg(theme.bg);

        let list = List::new(theme_names)
            .block(block)
            .scroll_padding(area.height as usize - 3)
            .fg(theme.text_muted)
            .highlight_symbol(SELECTOR)
            .highlight_style(theme.accent);

        StatefulWidget::render(list, area, buf, &mut state.popup.selection);
    }
}
