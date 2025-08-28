use crate::ui_state::{AlbumSort, Pane, UiState, GOLD_FADED};
use ratatui::{
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{
        Block, BorderType, HighlightSpacing, List, ListItem, ListState, Padding, StatefulWidget,
    },
};

// album_view.rs
pub struct SideBarAlbum;
impl StatefulWidget for SideBarAlbum {
    type State = UiState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let theme = &state.get_theme(&Pane::SideBar);

        let albums = &state.albums;
        let pane_sort = state.get_album_sort_string();
        let pane_sort = format!("{pane_sort:5} ");

        let selected_album_idx = state.display_state.album_pos.selected();
        let selected_artist = state.get_selected_album().map(|a| a.artist.as_str());

        let mut list_items = Vec::new();
        let mut current_artist = None;
        let mut current_display_idx = 0;
        let mut selected_display_idx = None;

        for (idx, album) in albums.iter().enumerate() {
            // Add header if artist changed (only for Artist sort)
            if state.get_album_sort() == AlbumSort::Artist {
                if current_artist.as_ref() != Some(&album.artist.as_str()) {
                    let artist_str = album.artist.as_str();
                    let is_selected_artist = selected_artist == Some(artist_str);

                    // Match header style to selected album
                    let header_style = match is_selected_artist {
                        true => Style::default()
                            .fg(theme.text_highlighted)
                            .italic()
                            .underlined(),
                        false => Style::default().fg(GOLD_FADED),
                    };

                    list_items.push(ListItem::new(Span::from(artist_str).style(header_style)));

                    current_artist = Some(artist_str);
                    current_display_idx += 1;
                }
            }

            // Build album item
            let year = album.year.map_or("----".to_string(), |y| format!("{y}"));

            let indent = match state.get_album_sort() == AlbumSort::Artist {
                true => "  ",
                false => "",
            };

            let is_selected = selected_album_idx == Some(idx);
            if is_selected {
                selected_display_idx = Some(current_display_idx);
            }

            // Don't apply selection styling here - let the List widget handle it
            list_items.push(ListItem::new(Line::from_iter([
                Span::from(format!("{}{: >4} ", indent, year)).fg(theme.text_secondary),
                Span::from("✧ ").fg(theme.text_faded),
                Span::from(album.title.as_str()).fg(theme.text_focused),
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

        let keymaps = match state.get_pane() {
            Pane::SideBar => Line::from(" [q] Queue Album ")
                .centered()
                .fg(theme.text_faded),
            _ => Line::default(),
        };

        let block = Block::bordered()
            .borders(theme.border_display)
            .border_type(BorderType::Thick)
            .border_style(theme.border)
            .bg(theme.bg)
            .title_top(format!(" ⟪ {} Albums! ⟫ ", albums.len()))
            .title_top(
                Line::from_iter([" 󰒿 ", &pane_sort])
                    .right_aligned()
                    .fg(theme.text_secondary),
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
            .scroll_padding(5)
            .highlight_spacing(HighlightSpacing::Always);

        list.render(area, buf, &mut render_state);

        // Sync offset back
        *state.display_state.album_pos.offset_mut() = render_state.offset();
    }
}
