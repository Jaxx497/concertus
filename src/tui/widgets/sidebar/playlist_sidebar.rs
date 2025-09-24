use ratatui::{
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, List, ListItem, Padding, StatefulWidget},
};

use crate::ui_state::{GOLD_FADED, Pane, UiState};

pub struct SideBarPlaylist;
impl StatefulWidget for SideBarPlaylist {
    type State = UiState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let theme = &state.get_theme(&Pane::SideBar);
        let playlists = &state.playlists;

        let list_items = playlists.iter().map(|p| {
            ListItem::new(
                Line::from_iter([
                    Span::from(p.name.as_str()).fg(theme.text_secondary),
                    format!("{:>5} ", format!("[{}]", p.tracklist.len()))
                        .fg(GOLD_FADED)
                        .into(),
                ])
                .right_aligned(),
            )
        });

        let keymaps = match state.get_pane() {
            Pane::SideBar => Line::from(" [c]reate 󰲸 | [D]elete 󰐓 ")
                .centered()
                .fg(theme.text_faded),
            _ => Line::default(),
        };

        let block = Block::bordered()
            .borders(theme.border_display)
            .border_type(theme.border_type)
            .border_style(theme.border)
            .bg(theme.bg_panel)
            .title_top(
                Line::from(format!(" ⟪ {} Playlists! ⟫ ", playlists.len()))
                    .left_aligned()
                    .fg(theme.text_highlighted),
            )
            .title_bottom(Line::from(keymaps).centered().fg(theme.text_faded))
            .padding(Padding {
                left: 3,
                right: 4,
                top: 1,
                bottom: 1,
            });

        let list = List::new(list_items)
            .block(block)
            .highlight_style(
                Style::new()
                    .fg(Color::Black)
                    .bg(theme.text_highlighted)
                    .italic(),
            )
            .scroll_padding(4);

        list.render(area, buf, &mut state.display_state.playlist_pos);
    }
}
