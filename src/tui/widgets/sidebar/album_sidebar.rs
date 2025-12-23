use crate::{
    tui::widgets::sidebar::create_standard_list,
    ui_state::{AlbumSort, Pane, UiState},
};
use ratatui::{
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{ListItem, ListState, StatefulWidget},
};

pub struct SideBarAlbum;
impl StatefulWidget for SideBarAlbum {
    type State = UiState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let focus = matches!(&state.get_pane(), Pane::SideBar);
        let theme = &state.theme_manager.get_display_theme(focus);

        let albums = &state.albums;
        let pane_sort = state.get_album_sort_string();
        let pane_sort = format!(" 󰒿 {pane_sort:5} ");
        let album_sort = state.get_album_sort();

        let selected_album_idx = state.display_state.album_pos.selected();
        let selected_artist = state.get_selected_album().map(|a| a.artist.as_str());

        let mut list_items = Vec::with_capacity(albums.len());
        let mut current_artist = None;
        let mut current_display_idx = 0;
        let mut selected_display_idx = None;

        for (idx, album) in albums.iter().enumerate() {
            // Add header if artist changed (only for Artist sort)

            if album_sort == AlbumSort::Artist {
                if current_artist.as_ref() != Some(&album.artist.as_str()) {
                    let artist_str = album.artist.as_str();
                    let is_selected_artist = selected_artist == Some(artist_str);

                    let header_style = match is_selected_artist {
                        true => Style::default().fg(theme.text_secondary).underlined(),
                        false => Style::default().fg(theme.text_secondary),
                    };

                    list_items.push(ListItem::new(Span::from(artist_str).style(header_style)));

                    current_artist = Some(artist_str);
                    current_display_idx += 1;
                }
            }

            let year = album.year.map_or("????".to_string(), |y| format!("{y}"));
            let year_color = match album_sort {
                AlbumSort::Artist => theme.text_muted,
                _ => theme.text_secondary,
            };

            let indent = match state.get_album_sort() == AlbumSort::Artist {
                true => "  ",
                false => "",
            };

            let is_selected = selected_album_idx == Some(idx);
            if is_selected {
                selected_display_idx = Some(current_display_idx);
            }
            let decorator = &state.get_decorator();

            let album_title = match album.title.is_empty() {
                true => album.artist.to_string() + " [Unknown Album]",
                false => album.title.to_string(),
            };

            list_items.push(ListItem::new(Line::from_iter([
                Span::from(format!("{}{: >4} ", indent, year)).fg(year_color),
                Span::from(format!("{decorator} ")).fg(theme.text_muted),
                Span::from(album_title).fg(theme.text_primary),
            ])));

            current_display_idx += 1;
        }

        // Temp state for rendering with display index
        let mut render_state = ListState::default();
        render_state.select(selected_display_idx);

        // Sync offset to ensure selection is visible
        if let Some(idx) = selected_display_idx {
            let current_offset = state.display_state.album_pos.offset();
            let visible_height = area.height.saturating_sub(4) as usize;

            if idx < current_offset {
                *render_state.offset_mut() = idx;
            } else if idx >= current_offset + visible_height {
                *render_state.offset_mut() = idx.saturating_sub(visible_height - 1);
            } else {
                *render_state.offset_mut() = current_offset;
            }
        }

        let title = Line::from(format!(" ⟪ {} Albums ⟫ ", albums.len())).fg(theme.accent);
        let sorting = Line::from(pane_sort)
            .right_aligned()
            .fg(theme.text_secondary);

        create_standard_list(list_items, (title, sorting), state, area).render(
            area,
            buf,
            &mut render_state,
        );

        // Sync offset back
        *state.display_state.album_pos.offset_mut() = render_state.offset();
    }
}
