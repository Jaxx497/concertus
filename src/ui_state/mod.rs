mod album_sort;
mod mode;
mod pane;
mod playback;
mod search_state;
mod settings;
mod table_sort;
mod theme;
mod ui_snapshot;
mod ui_state;

pub use album_sort::AlbumSort;
pub use mode::Mode;
pub use pane::Pane;
pub use settings::SettingsMode;
pub use table_sort::TableSort;
pub use theme::DisplayTheme;
pub use ui_snapshot::UiSnapshot;
pub use ui_state::UiState;

pub use theme::*;

pub enum AlbumDisplayItem {
    Header(String),
    Album(usize),
}

fn new_textarea(placeholder: &str) -> tui_textarea::TextArea<'static> {
    let mut search = tui_textarea::TextArea::default();
    search.set_cursor_line_style(ratatui::style::Style::default());
    search.set_placeholder_text(format!(" {placeholder}: "));

    search
}
