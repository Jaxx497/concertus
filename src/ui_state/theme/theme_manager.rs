use anyhow::anyhow;

use crate::{
    CONFIG_DIRECTORY, THEME_DIRECTORY,
    key_handler::MoveDirection,
    ui_state::{PopupType, ThemeConfig, UiState},
};

pub struct ThemeManager {
    pub active: ThemeConfig,
    pub theme_lib: Vec<ThemeConfig>,
}

impl ThemeManager {
    pub fn new() -> Self {
        let theme_lib = Self::collect_themes();
        let active = theme_lib.first().cloned().unwrap_or_default();

        ThemeManager { active, theme_lib }
    }

    pub fn get_themes(&self) -> Vec<ThemeConfig> {
        self.theme_lib.clone()
    }

    pub fn update_themes(&mut self) {
        let themes = Self::collect_themes();
        self.theme_lib = themes
    }

    pub fn find_theme_by_name(&self, name: &str) -> Option<&ThemeConfig> {
        self.theme_lib.iter().find(|t| t.name == name)
    }

    pub fn get_current_theme_index(&self) -> Option<usize> {
        self.theme_lib
            .iter()
            .position(|t| t.name == self.active.name)
    }

    pub fn get_theme_at_index(&self, idx: usize) -> Option<ThemeConfig> {
        self.theme_lib.get(idx).cloned()
    }

    pub fn set_theme(&mut self, theme: ThemeConfig) {
        self.active = theme
    }

    fn collect_themes() -> Vec<ThemeConfig> {
        let mut themes = vec![];
        let theme_dir =
            dirs::config_dir().map(|dir| dir.join(CONFIG_DIRECTORY).join(THEME_DIRECTORY));

        if let Some(ref theme_path) = theme_dir {
            let _ = std::fs::create_dir_all(theme_path);

            if let Ok(entries) = theme_path.read_dir() {
                for entry in entries.flatten() {
                    let path = entry.path();

                    if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                        if let Ok(theme) = ThemeConfig::load_from_file(&path) {
                            themes.push(theme);
                        }
                    }
                }
            }
        }
        themes
    }
}

impl UiState {
    pub fn refresh_current_theme(&mut self) {
        self.theme_manager.update_themes();

        match self.theme_manager.get_current_theme_index() {
            Some(idx) => {
                let theme = self
                    .theme_manager
                    .get_theme_at_index(idx)
                    .unwrap_or_default();
                self.theme_manager.set_theme(theme);
            }
            _ => self.set_error(anyhow!(
                "Formatting error in theme!\n\nFalling back to last loaded"
            )),
        }
    }

    pub fn open_theme_manager(&mut self) {
        self.theme_manager.update_themes();

        if let Some(idx) = self.theme_manager.get_current_theme_index() {
            let theme = self
                .theme_manager
                .get_theme_at_index(idx)
                .unwrap_or_default();

            self.theme_manager.set_theme(theme);
            self.popup.selection.select(Some(idx));
        }

        self.show_popup(PopupType::ThemeManager);
    }

    pub fn cycle_theme(&mut self, dir: MoveDirection) {
        let len = self.theme_manager.theme_lib.len();
        if len < 2 {
            return;
        }

        let idx = self.theme_manager.get_current_theme_index().unwrap_or(0);
        let new_idx = match dir {
            MoveDirection::Up => (idx + len - 1) % len,
            MoveDirection::Down => (idx + 1) % len,
        };

        self.theme_manager.active = self
            .theme_manager
            .theme_lib
            .get(new_idx)
            .cloned()
            .unwrap_or_default()
    }
}
