use crate::{
    Library,
    app_core::Concertus,
    ui_state::{PopupType, SettingsMode, UiState},
};
use anyhow::{Result, anyhow};
use std::sync::Arc;

impl UiState {
    pub fn get_settings_mode(&self) -> Option<&SettingsMode> {
        match &self.popup.current {
            PopupType::Settings(mode) => Some(mode),
            _ => None,
        }
    }

    pub fn get_roots(&self) -> Vec<String> {
        let mut roots: Vec<String> = self
            .library
            .roots
            .iter()
            .map(|p| p.display().to_string())
            .collect();
        roots.sort();
        roots
    }

    pub fn add_root(&mut self, path: &str) -> Result<()> {
        let mut lib = Library::init();
        lib.add_root(path)?;
        lib.build_library()?;

        self.library = Arc::new(lib);

        Ok(())
    }

    pub fn remove_root(&mut self) -> Result<()> {
        if let Some(selected) = self.popup.selection.selected() {
            let roots = self.get_roots();
            if selected >= roots.len() {
                return Err(anyhow!("Invalid root index!"));
            }

            let mut lib = Library::init();

            let bad_root = &roots[selected];
            lib.delete_root(&bad_root)?;
        }

        Ok(())
    }

    pub fn enter_settings(&mut self) {
        if !self.get_roots().is_empty() {
            self.popup.selection.select(Some(0));
        }

        self.show_popup(PopupType::Settings(SettingsMode::ViewRoots));
    }
}

impl Concertus {
    pub(crate) fn settings_remove_root(&mut self) {
        if !self.ui.get_roots().is_empty() {
            self.ui
                .show_popup(PopupType::Settings(SettingsMode::RemoveRoot));
        }
    }

    pub(crate) fn activate_settings(&mut self) {
        match self.ui.get_roots().is_empty() {
            true => self.ui.popup.selection.select(None),
            false => self.ui.popup.selection.select(Some(0)),
        }
        self.ui
            .show_popup(PopupType::Settings(SettingsMode::ViewRoots))
    }

    pub(crate) fn popup_scroll_up(&mut self) {
        let list_len = match self.ui.popup.current {
            PopupType::Settings(_) => self.ui.get_roots().len(),
            PopupType::Playlist(_) => self.ui.playlists.len(),
            _ => return,
        };

        if list_len > 0 {
            let current = self.ui.popup.selection.selected().unwrap_or(0);
            let new_selection = if current > 0 {
                current - 1
            } else {
                list_len - 1 // Wrap to bottom
            };
            self.ui.popup.selection.select(Some(new_selection));
        }
    }

    pub(crate) fn popup_scroll_down(&mut self) {
        let list_len = match self.ui.popup.current {
            PopupType::Settings(_) => self.ui.get_roots().len(),
            PopupType::Playlist(_) => self.ui.playlists.len(),
            _ => return,
        };

        if list_len > 0 {
            let current = self.ui.popup.selection.selected().unwrap_or(0);
            let new_selection = (current + 1) % list_len; // Wrap to top
            self.ui.popup.selection.select(Some(new_selection));
        }
    }

    pub(crate) fn settings_add_root(&mut self) {
        self.ui
            .show_popup(PopupType::Settings(SettingsMode::AddRoot));
    }

    pub(crate) fn settings_root_confirm(&mut self) -> anyhow::Result<()> {
        match self.ui.popup.current {
            PopupType::Settings(SettingsMode::AddRoot) => {
                let path = self.ui.get_popup_string();
                if !path.is_empty() {
                    match self.ui.add_root(&path) {
                        Err(e) => self.ui.set_error(e),
                        Ok(_) => {
                            self.update_library()?;
                            self.ui.close_popup();
                        }
                    }
                }
            }
            PopupType::Settings(SettingsMode::RemoveRoot) => {
                if let Err(e) = self.ui.remove_root() {
                    self.ui.set_error(e);
                } else {
                    self.ui
                        .show_popup(PopupType::Settings(SettingsMode::ViewRoots));
                    self.ui.popup.selection.select(Some(0));
                    self.update_library()?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
