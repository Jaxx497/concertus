mod album_sidebar;
mod handler;
mod playlist_sidebar;

pub use album_sidebar::SideBarAlbum;
pub use handler::SideBarHandler;
pub use playlist_sidebar::SideBarPlaylist;
use ratatui::{
    layout::Rect,
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, HighlightSpacing, List, ListItem, Padding},
};

use crate::ui_state::{LibraryView, Pane, UiState};

const PADDING: Padding = Padding {
    left: 3,
    right: 2,
    top: 1,
    bottom: 1,
};

pub fn create_standard_list<'a>(
    list_items: Vec<ListItem<'a>>,
    titles: (Line<'static>, Line<'static>),
    state: &UiState,
    area: Rect,
) -> List<'a> {
    let focus = matches!(&state.get_pane(), Pane::SideBar);
    let theme = state.theme_manager.get_display_theme(focus);

    let keymaps = if state.get_pane() == Pane::SideBar {
        match state.display_state.sidebar_view {
            LibraryView::Albums => Line::from(" [q] Queue Album ")
                .centered()
                .fg(theme.text_muted),
            LibraryView::Playlists => {
                let playlist_keymaps = " [c]reate 󰲸 | [^D]elete 󰐓 ";
                match area.width as usize + 2 < playlist_keymaps.len() {
                    true => Line::default(),
                    false => Line::from(playlist_keymaps).centered().fg(theme.text_muted),
                }
            }
        }
    } else {
        Line::default()
    };

    let block = Block::bordered()
        .borders(theme.border_display)
        .border_type(theme.border_type)
        .border_style(theme.border)
        .bg(theme.bg)
        .title_top(titles.0)
        .title_top(titles.1)
        .title_bottom(Line::from(keymaps).centered().fg(theme.text_muted))
        .padding(PADDING);

    List::new(list_items)
        .block(block)
        .highlight_style(Style::new().fg(theme.text_selected).bg(theme.selection))
        .scroll_padding((area.height as f32 * 0.15) as usize)
        .highlight_spacing(HighlightSpacing::Always)
}
