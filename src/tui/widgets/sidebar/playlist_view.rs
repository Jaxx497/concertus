use ratatui::{
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, List, ListItem, Padding, StatefulWidget},
};

use crate::ui_state::{Pane, UiState};

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

        let list_items = playlists.iter().map(|p| ListItem::new(Span::from(&p.name)));

        let pane_title = format!(" ⟪ {} Playlists! ⟫ ", playlists.len());

        let keymaps = match state.get_pane() {
            Pane::SideBar => Line::from("[c] Create, [d] Delete")
                .centered()
                .fg(theme.text_faded),
            _ => Line::default(),
        };

        let block = Block::bordered()
            .border_type(BorderType::Thick)
            .border_style(theme.border)
            .bg(theme.bg)
            .title_top(Line::from(pane_title).left_aligned().fg(theme.text_focused))
            // .title_top(
            //     Line::from_iter([" 󰒿 ", &pane_org])
            //         .right_aligned()
            //         .fg(theme.text_secondary),
            // )
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

        list.render(area, buf, &mut state.display_state.sidebar_pos);
    }
}
