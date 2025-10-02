mod layout;
mod renderer;
mod widgets;

pub use layout::AppLayout;
pub use renderer::render;
pub use widgets::{ErrorMsg, Progress, SearchBar, SideBarHandler as SideBar, SongTable};

use ratatui::widgets::Padding;
pub(crate) const SEARCH_PADDING: Padding = Padding {
    left: 2,
    right: 2,
    top: 1,
    bottom: 0,
};
