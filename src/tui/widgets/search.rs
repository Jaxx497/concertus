use crate::ui_state::{Pane, UiState};
use ratatui::{
    style::Stylize,
    widgets::{Block, Borders, Padding, StatefulWidget, Widget},
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
        let focus = matches!(&state.get_pane(), Pane::Search);
        let theme = &state.theme_manager.get_display_theme(focus);
        let (border_display, border_type, border_style, accent, bg) = {
            (
                theme.border_display,
                theme.border_type,
                theme.border,
                theme.border,
                theme.bg,
            )
        };

        let (pd_v, pd_h) = match theme.border_display {
            Borders::NONE => (2, 3),
            _ => (1, 2),
        };

        // let search = &mut state.search.input;
        let search = state.get_search_widget();
        search.set_block(
            Block::bordered()
                .borders(border_display)
                .border_type(border_type)
                .border_style(border_style)
                .padding(Padding {
                    left: pd_h,
                    right: 0,
                    top: pd_v,
                    bottom: 0,
                })
                .fg(accent)
                .bg(bg),
        );

        search.render(area, buf);
    }
}
