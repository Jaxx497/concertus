mod buffer_line;
mod error;
mod playlist_popup;
mod progress;
mod progress_bar;
mod search;
mod settings;
mod sidebar;
mod song_window;
mod tracklist;
mod waveform;

pub use buffer_line::BufferLine;
pub use error::ErrorMsg;
pub use playlist_popup::PlaylistPopup;
pub use progress::Progress;
pub use search::SearchBar;
pub use settings::Settings;
pub use sidebar::SideBarHandler;
pub use song_window::SongTable;
pub use waveform::Waveform;

const DUR_WIDTH: u16 = 5;
const PAUSE_ICON: &str = "󰏤";
const WAVEFORM_WIDGET_HEIGHT: f64 = 50.0;

static POPUP_PADDING: ratatui::widgets::Padding = ratatui::widgets::Padding {
    left: 2,
    right: 2,
    top: 1,
    bottom: 1,
};
