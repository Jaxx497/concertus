mod display_state;
mod domain;
mod multi_select;
mod playlist;
mod popup;
mod progress_display;
mod search_state;
mod settings;
mod theme;
mod ui_snapshot;
mod ui_state;
mod waveform;

use std::sync::Arc;

pub use display_state::DisplayState;
pub use domain::{AlbumSort, LibraryView, Mode, Pane, TableSort};
pub use playlist::PlaylistAction;
pub use popup::PopupType;
pub use progress_display::ProgressDisplay;
pub use search_state::MatchField;
pub use settings::SettingsMode;
pub use theme::DisplayTheme;
pub use ui_snapshot::UiSnapshot;
pub use waveform::WaveformManager;

use crate::{
    database::DbWorker,
    domain::{Album, Playlist, SimpleSong},
    player::PlaybackMetrics,
    ui_state::{popup::PopupState, search_state::SearchState},
    Library, PlaybackSession,
};

pub struct UiState {
    library: Arc<Library>,
    db_worker: DbWorker,

    metrics: Arc<PlaybackMetrics>,
    pub(crate) playback: PlaybackSession,

    search: SearchState,
    pub(crate) popup: PopupState,
    pub(crate) theme_manager: ThemeManager,
    pub(crate) display_state: DisplayState,

    waveform: WaveformManager,
    progress_display: ProgressDisplay,

    legal_songs: Vec<Arc<SimpleSong>>,
    pub(crate) albums: Vec<Album>,
    pub(crate) playlists: Vec<Playlist>,

    pub library_refresh_progress: Option<u8>,
    pub library_refresh_detail: Option<String>,
}

pub use theme::*;

fn new_textarea(placeholder: &str) -> tui_textarea::TextArea<'static> {
    let mut search = tui_textarea::TextArea::default();
    search.set_cursor_line_style(ratatui::style::Style::default());
    search.set_placeholder_text(format!(" {placeholder}: "));

    search
}
