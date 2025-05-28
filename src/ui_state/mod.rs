mod album_sort;
mod mode;
mod pane;
mod table_sort;
mod theme;
mod ui_state;

pub use album_sort::AlbumSort;
pub use mode::Mode;
pub use pane::Pane;
pub use table_sort::TableSort;
pub use theme::DisplayTheme;
pub use ui_state::UiState;

pub use theme::GOOD_RED;
pub use ui_state::SettingsMode;

const HISTORY_CAPACITY: usize = 50;
const MATCH_THRESHOLD: i64 = 50;
static MATCHER: std::sync::LazyLock<fuzzy_matcher::skim::SkimMatcherV2> =
    std::sync::LazyLock::new(|| fuzzy_matcher::skim::SkimMatcherV2::default());

pub enum AlbumDisplayItem {
    Header(String),
    Album(usize),
}
