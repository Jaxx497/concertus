mod buffer_line;
mod popup;
mod popups;
mod progress;
mod search;
mod sidebar;
mod song_window;
mod tracklist;

pub use buffer_line::BufferLine;
pub use popup::PopupManager;
pub use popups::{ErrorMsg, PlaylistPopup, RootManager, ThemeManager};
pub use progress::Progress;
pub use search::SearchBar;
pub use sidebar::SideBarHandler;
pub use song_window::SongTable;

const DUR_WIDTH: u16 = 5;
const PAUSE_ICON: &str = "󰏤";
const SELECTOR: &str = "⮞  ";
const QUEUE_ICON: &str = "󰐑";
const MUSIC_NOTE: &str = "♫";
const QUEUED: &str = "";
const SELECTED: &str = "󱕣";
const WAVEFORM_WIDGET_HEIGHT: f64 = 50.0;

static POPUP_PADDING: ratatui::widgets::Padding = ratatui::widgets::Padding {
    left: 5,
    right: 5,
    top: 2,
    bottom: 2,
};
