use crate::ui_state::UiState;
use ratatui::{
    layout::{Alignment, Constraint, Layout},
    style::{Color, Stylize},
    widgets::{Block, Padding, Paragraph, StatefulWidget, Widget, Wrap},
};

static SIDE_PADDING: u16 = 5;
static VERTICAL_PADDING: u16 = 1;

static PADDING: Padding = Padding {
    left: SIDE_PADDING,
    right: SIDE_PADDING,
    top: VERTICAL_PADDING,
    bottom: VERTICAL_PADDING,
};

pub struct ErrorMsg;
impl StatefulWidget for ErrorMsg {
    type State = UiState;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let theme = &state.theme_manager.get_display_theme(true);

        let block = Block::bordered()
            .border_type(theme.border_type)
            .title_bottom(" Press <Esc> to clear ")
            .title_alignment(Alignment::Center)
            .padding(PADDING)
            .fg(Color::White)
            .bg(theme.bg_error);

        let inner = block.inner(area);
        block.render(area, buf);
        let chunks = Layout::vertical([
            Constraint::Percentage(33),
            Constraint::Length(3),
            Constraint::Fill(1),
        ])
        .split(inner);

        let err_str = state.get_error().unwrap_or("No error to display");

        Paragraph::new(err_str)
            .wrap(Wrap { trim: true })
            .centered()
            .render(chunks[1], buf);
    }
}
