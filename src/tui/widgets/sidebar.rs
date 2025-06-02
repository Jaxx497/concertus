use crate::ui_state::{AlbumDisplayItem, AlbumSort, Pane, UiState};
use ratatui::{
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, List, ListItem, ListState, Padding, StatefulWidget},
};

pub struct SideBar;
impl StatefulWidget for SideBar {
    type State = UiState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let albums = &state.filtered_albums;
        let pane_title = format!(" ⟪ {} Albums! ⟫ ", albums.len());
        let pane_org = state.get_album_sort_string();
        let pane_org = format!("{pane_org:5} ");

        let theme = &state.get_theme(&Pane::SideBar);

        // Get the currently selected artist (if any)
        let selected_artist = state
            .album_pos
            .selected()
            .and_then(|idx| albums.get(idx))
            .map(|album| album.artist.as_str());

        // Create list items from display items
        let display_items = &state.album_display_items;

        let list_items = display_items
            .iter()
            .map(|item| match item {
                AlbumDisplayItem::Header(artist) => {
                    let is_selected_artist = selected_artist.map_or(false, |sel| sel == artist);

                    let style = match is_selected_artist {
                        true => Style::default()
                            .fg(theme.text_highlighted)
                            .add_modifier(Modifier::ITALIC | Modifier::UNDERLINED),
                        false => Style::default().fg(theme.text_faded),
                    };

                    ListItem::new(Span::from(format!("{}", artist)).italic().style(style))
                }
                AlbumDisplayItem::Album(idx) => {
                    let album = &albums[*idx];

                    let year = match album.year {
                        Some(y) => format!("{y}"),
                        _ => String::from("----"),
                    };

                    let indent = match state.get_album_sort() == AlbumSort::Artist {
                        true => "  ",
                        false => "",
                    };

                    let year_txt =
                        Span::from(format!("{}{: >4} ", indent, year)).fg(theme.text_secondary);
                    let separator = Span::from("✧ ").fg(theme.text_faded);
                    let album_title = Span::from(album.title.as_str()).fg(theme.text_focused);

                    ListItem::new(Line::from_iter([year_txt, separator, album_title]))
                }
            })
            .collect::<Vec<ListItem>>();

        let display_selected = if let Some(album_idx) = state.album_pos.selected() {
            state
                .album_display_items
                .iter()
                .position(|item| match item {
                    AlbumDisplayItem::Album(idx) => *idx == album_idx,
                    _ => false,
                })
        } else {
            None
        };

        // Create a temporary display state
        let mut display_state = ListState::default();
        display_state.select(display_selected);
        *display_state.offset_mut() = state.album_pos.offset();

        let current_offset = state.album_pos.offset();
        *display_state.offset_mut() = current_offset;

        // Ensure header is visible
        if state.get_album_sort() == AlbumSort::Artist && display_selected.is_some() {
            let display_idx = display_selected.unwrap();

            // Get album header
            let mut header_idx = display_idx;
            while header_idx > 0 {
                header_idx -= 1;
                if let AlbumDisplayItem::Header(_) = display_items[header_idx] {
                    break;
                }
            }

            if header_idx < current_offset && display_idx >= current_offset {
                *display_state.offset_mut() = header_idx;
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
            .title_top(Line::from(pane_title).left_aligned().fg(theme.text_focused))
            .title_top(
                Line::from_iter([" 󰒿 ", &pane_org])
                    .right_aligned()
                    .fg(theme.text_secondary),
            )
            .title_bottom(Line::from(keymaps).centered().fg(theme.text_faded))
            .padding(get_padding(state.get_pane()));

        let list = List::new(list_items)
            .block(block)
            .highlight_style(Style::new().fg(Color::Black).bg(theme.text_highlighted))
            .scroll_padding(4);

        list.render(area, buf, &mut display_state);
        *state.album_pos.offset_mut() = display_state.offset();
    }
}

fn get_padding(pane: &Pane) -> Padding {
    match pane {
        Pane::SideBar => Padding {
            left: 3,
            right: 4,
            top: 2,
            bottom: 1,
        },
        _ => Padding {
            left: 4,
            right: 5,
            top: 2,
            bottom: 1,
        },
    }
}
