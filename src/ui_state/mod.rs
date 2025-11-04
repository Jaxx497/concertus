mod display_state;
mod domain;
mod multi_select;
mod playback;
mod playlist;
mod popup;
mod search_state;
mod settings;
mod theme;
mod ui_snapshot;
mod ui_state;

pub use display_state::DisplayState;
pub use domain::{AlbumSort, LibraryView, Mode, Pane, TableSort};
pub use playback::{PlaybackView, ProgressDisplay};
pub use playlist::PlaylistAction;
pub use popup::PopupType;
pub use search_state::MatchField;
pub use settings::SettingsMode;
pub use theme::DisplayTheme;
pub use ui_snapshot::UiSnapshot;
pub use ui_state::UiState;

pub use theme::*;

fn new_textarea(placeholder: &str) -> tui_textarea::TextArea<'static> {
    let mut search = tui_textarea::TextArea::default();
    search.set_cursor_line_style(ratatui::style::Style::default());
    search.set_placeholder_text(format!(" {placeholder}: "));

    search
}
