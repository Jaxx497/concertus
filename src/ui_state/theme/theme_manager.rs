use anyhow::anyhow;

use crate::{
    key_handler::Incrementor,
    ui_state::{fade_color, DisplayTheme, PopupType, ThemeConfig, UiState},
    CONFIG_DIRECTORY, THEME_DIRECTORY,
};

pub struct ThemeManager {
    pub active: ThemeConfig,
    pub cached_focused: DisplayTheme,
    pub cached_unfocused: DisplayTheme,

    pub theme_lib: Vec<ThemeConfig>,
}

impl ThemeManager {
    pub fn new() -> Self {
        let theme_lib = Self::collect_themes();
        let active = theme_lib.first().cloned().unwrap_or_default();

        let cached_focused = Self::set_display_theme(&active, true);
        let cached_unfocused = Self::set_display_theme(&active, false);

        ThemeManager {
            active,
            theme_lib,
            cached_focused,
            cached_unfocused,
        }
    }

    pub fn set_theme(&mut self, theme: ThemeConfig) {
        self.cached_focused = Self::set_display_theme(&theme, true);
        self.cached_unfocused = Self::set_display_theme(&theme, false);
        self.active = theme;
    }

    pub fn get_display_theme(&self, focus: bool) -> &DisplayTheme {
        match focus {
            true => &self.cached_focused,
            false => &self.cached_unfocused,
        }
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

    fn set_display_theme(theme: &ThemeConfig, focused: bool) -> DisplayTheme {
        let is_dark = theme.is_dark;

        let progress_played = theme.progress_played.to_owned();
        let progress_unplayed = theme.progress_unplayed.to_owned();
        let progress_speed = theme.progress_speed;
        let bar_active = theme.bar_active.to_string();
        let bar_inactive = theme.bar_inactive.to_string();
        let waveform_style = theme.waveform_style;
        let oscilloscope_style = theme.oscilloscope_style;

        match focused {
            true => DisplayTheme {
                dark: theme.is_dark,
                bg: theme.surface_active,
                bg_global: theme.surface_global,
                bg_error: theme.surface_error,

                text_primary: theme.text_primary,
                text_secondary: theme.text_secondary,
                text_muted: theme.text_muted,
                text_selected: theme.text_selection,

                selection: theme.selection,

                accent: theme.accent,

                border: theme.border_active,
                border_display: theme.border_display,
                border_type: theme.border_type,

                progress_played,
                progress_unplayed,
                progress_speed,
                bar_active,
                bar_inactive,
                waveform_style,
                oscilloscope_style,
            },

            false => DisplayTheme {
                dark: theme.is_dark,
                bg: theme.surface_inactive,
                bg_global: theme.surface_global,
                bg_error: theme.surface_error,

                text_primary: theme.text_muted,
                text_secondary: theme.text_secondary_in,
                text_muted: fade_color(is_dark, theme.text_muted, 0.7),
                text_selected: theme.text_selection,

                selection: theme.selection_inactive,
                accent: theme.accent_inactive,

                border: theme.border_inactive,
                border_display: theme.border_display,
                border_type: theme.border_type,

                progress_played,
                progress_unplayed,
                progress_speed,
                bar_active,
                bar_inactive,
                waveform_style,
                oscilloscope_style,
            },
        }
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

    pub fn cycle_theme(&mut self, dir: Incrementor) {
        let len = self.theme_manager.theme_lib.len();
        if len < 2 {
            return;
        }

        let idx = self.theme_manager.get_current_theme_index().unwrap_or(0);
        let new_idx = match dir {
            Incrementor::Up => (idx + len - 1) % len,
            Incrementor::Down => (idx + 1) % len,
        };

        self.theme_manager.set_theme(
            self.theme_manager
                .theme_lib
                .get(new_idx)
                .cloned()
                .unwrap_or_default(),
        );
    }
}
