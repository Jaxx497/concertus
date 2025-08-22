// tui/widgets/playlist_popup.rs
use crate::ui_state::{PlaylistAction, PopupType, UiState};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style, Stylize},
    widgets::{Block, BorderType, Padding, Paragraph, StatefulWidget, Widget},
};

pub struct PlaylistPopup;
impl StatefulWidget for PlaylistPopup {
    type State = UiState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        if let PopupType::Playlist(action) = &state.popup.current {
            match action {
                PlaylistAction::Create => render_create_popup(area, buf, state),
                _ => (),
            }
        }
    }
}

fn render_create_popup(
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
    state: &mut UiState,
) {
    let block = Block::bordered()
        .title(" Create New Playlist ")
        .title_bottom(" [Enter] confirm / [Esc] cancel ")
        .title_alignment(ratatui::layout::Alignment::Center)
        .border_type(BorderType::Double)
        .border_style(Style::new().fg(Color::Rgb(255, 70, 70)))
        .bg(Color::Rgb(25, 25, 25))
        .padding(Padding {
            left: 2,
            right: 2,
            top: 1,
            bottom: 1,
        });

    let inner = block.inner(area);
    block.render(area, buf);

    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(3),
        Constraint::Min(1),
    ])
    .split(inner);

    Paragraph::new("Enter a name for your new playlist:").render(chunks[0], buf);

    state.popup.input.set_block(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .fg(Color::Rgb(220, 220, 100))
            .padding(Padding::horizontal(1)),
    );
    state.popup.input.set_style(Style::new().fg(Color::White));
    state.popup.input.render(chunks[1], buf);

    Paragraph::new("Tip: Choose a descriptive name like 'Workout Mix' or 'Chill Vibes'")
        .fg(Color::DarkGray)
        .centered()
        .render(chunks[2], buf);
}
