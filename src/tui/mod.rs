mod layout;
mod renderer;
mod widgets;

use crate::ui_state::UiState;
use ratatui::{
    style::Stylize,
    widgets::{Block, Widget},
};

pub use layout::AppLayout;
pub use renderer::render;
pub use widgets::{ErrorMsg, Progress, SearchBar, SideBarHandler as SideBar, SongTable};

pub fn render_bg(state: &UiState, f: &mut ratatui::Frame) {
    Block::new()
        .bg(state.theme_manager.active.bg.1)
        .render(f.area(), f.buffer_mut());
}
