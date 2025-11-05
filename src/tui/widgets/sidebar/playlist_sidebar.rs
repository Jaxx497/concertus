use ratatui::{
    layout::Alignment,
    style::Stylize,
    text::{Line, Span},
    widgets::{Block, Borders, ListItem, Padding, Paragraph, StatefulWidget, Widget, Wrap},
};

use crate::{
    tui::widgets::sidebar::create_standard_list,
    ui_state::{Pane, UiState},
};

pub struct SideBarPlaylist;
impl StatefulWidget for SideBarPlaylist {
    type State = UiState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let focus = matches!(&state.get_pane(), Pane::SideBar);
        let theme = state.theme_manager.get_display_theme(focus);
        let playlists = &state.playlists;

        if playlists.is_empty() {
            Widget::render(
                Paragraph::new("Create some playlists!\n\nPress [c] to get started!")
                    .block(Block::new().borders(Borders::NONE).padding(Padding {
                        left: 2,
                        right: 2,
                        top: 5,
                        bottom: 0,
                    }))
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: true })
                    .fg(theme.text_primary),
                area,
                buf,
            );
        }

        let list_items = playlists
            .iter()
            .map(|p| {
                ListItem::new(
                    Line::from_iter([
                        Span::from(p.name.as_str()).fg(theme.text_secondary),
                        format!("{:>5} ", format!("[{}]", p.tracklist.len()))
                            .fg(theme.text_secondary)
                            .into(),
                    ])
                    .right_aligned(),
                )
            })
            .collect();

        let title = Line::from(format!(" ⟪ {} Playlists ⟫ ", playlists.len()))
            .left_aligned()
            .fg(theme.accent);

        StatefulWidget::render(
            create_standard_list(list_items, (title, Line::default()), state, area),
            area,
            buf,
            &mut state.display_state.playlist_pos,
        );
    }
}
