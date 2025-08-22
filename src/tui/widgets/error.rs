use crate::ui_state::UiState;
use ratatui::{
    style::Stylize,
    widgets::{Block, BorderType, Padding, Paragraph, StatefulWidget, Widget, Wrap},
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
        let err_str = state.get_error().unwrap_or("No error to display");

        Paragraph::new(err_str)
            .wrap(Wrap { trim: true })
            .centered()
            .block(
                Block::bordered()
                    .border_type(BorderType::Double)
                    .title_bottom(" Press <Esc> to clear ")
                    .title_alignment(ratatui::layout::Alignment::Center)
                    .padding(PADDING),
            )
            .bg(ratatui::style::Color::LightRed)
            .render(area, buf);
    }
}
