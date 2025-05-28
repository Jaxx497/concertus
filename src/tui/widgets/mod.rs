mod error;
mod progress;
mod progress_bar;
mod search;
mod settings;
mod sidebar;
mod song_window;
mod waveform;

pub use error::ErrorMsg;
pub use progress::Progress;
pub use search::SearchBar;
pub use settings::Settings;
pub use sidebar::SideBar;
pub use song_window::SongTable;
pub use waveform::Waveform;

const DUR_WIDTH: u16 = 5;
const PAUSE_ICON: &str = "󰏤";
const SELECTOR: &str = "⮞  ";
const WAVEFORM_WIDGET_HEIGHT: f64 = 50.0;
