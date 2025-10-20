use crate::{
    key_handler::MoveDirection,
    ui_state::{PopupType, ThemeConfig, UiState},
    CONFIG_DIRECTORY, THEME_DIRECTORY,
};

pub struct ThemeManager {
    pub active: ThemeConfig,
    pub theme_lib: Vec<ThemeConfig>,
}

impl ThemeManager {
    pub fn new() -> Self {
        let theme_lib = Self::collect_themes();

        let active = theme_lib
            .first()
            .cloned()
            .unwrap_or_else(ThemeConfig::set_generic_theme);

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

    pub fn get_theme_index(&self) -> Option<usize> {
        self.theme_lib
            .iter()
            .position(|t| t.name == self.active.name)
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
    pub fn open_theme_manager(&mut self) {
        self.theme_manager.update_themes();

        let idx = self.theme_manager.get_theme_index();
        self.popup.selection.select(idx);

        self.show_popup(PopupType::ThemeManager);
    }

    pub fn cycle_theme(&mut self, dir: MoveDirection) {
        let len = self.theme_manager.theme_lib.len();
        if len == 0 {
            return;
        }

        let idx = self.theme_manager.get_theme_index().unwrap_or(0);
        let new_idx = match dir {
            MoveDirection::Up => (idx + len - 1) % len,
            MoveDirection::Down => (idx + 1) % len,
        };

        self.theme_manager.active = self
            .theme_manager
            .theme_lib
            .get(new_idx)
            .cloned()
            .unwrap_or(ThemeConfig::set_generic_theme())
    }
}
