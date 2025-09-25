use ratatui::{
    style::Stylize,
    text::{Line, Span},
    widgets::{ListItem, StatefulWidget},
};

use crate::{
    tui::widgets::sidebar::create_standard_list,
    ui_state::{GOLD_FADED, Pane, UiState},
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
        let theme = &state.get_theme(&Pane::SideBar);
        let playlists = &state.playlists;

        let list_items = playlists
            .iter()
            .map(|p| {
                ListItem::new(
                    Line::from_iter([
                        Span::from(p.name.as_str()).fg(theme.text_secondary),
                        format!("{:>5} ", format!("[{}]", p.tracklist.len()))
                            .fg(GOLD_FADED)
                            .into(),
                    ])
                    .right_aligned(),
                )
            })
            .collect();

        let title = Line::from(format!(" ⟪ {} Playlists ⟫ ", playlists.len()))
            .left_aligned()
            .fg(theme.text_highlighted);

        create_standard_list(list_items, (title, Line::default()), state, area).render(
            area,
            buf,
            &mut state.display_state.playlist_pos,
        );
    }
}
