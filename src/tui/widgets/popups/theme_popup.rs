use ratatui::{
    layout::Alignment,
    style::{Style, Stylize},
    widgets::{Block, BorderType, List, StatefulWidget},
};

use crate::{
    tui::widgets::POPUP_PADDING,
    ui_state::{Pane, UiState},
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
        let theme = state.get_theme(&Pane::Popup);

        let theme_names = state
            .theme_manager
            .theme_lib
            .iter()
            .map(|t| t.name.clone())
            .collect::<Vec<String>>();

        let block = Block::bordered()
            .title(" Select Theme ")
            .title_bottom(" [Enter] / [Esc] ")
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Double)
            .border_style(Style::new().fg(theme.text_secondary))
            .bg(theme.bg_panel)
            .padding(POPUP_PADDING);

        let list = List::new(theme_names)
            .block(block)
            .scroll_padding(area.height as usize - 3)
            .fg(theme.text_faded)
            .highlight_style(Style::new().fg(theme.highlight));

        StatefulWidget::render(list, area, buf, &mut state.popup.selection);
    }
}
