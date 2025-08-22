use crate::{
    app_core::Concertus,
    ui_state::{PopupType, SettingsMode, UiState},
    Library,
};
use anyhow::{anyhow, Result};
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
        let db = self.library.get_db();
        let mut lib = Library::init(db);
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

            let db = self.library.get_db();
            let mut lib = Library::init(db);

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
        self.ui
            .show_popup(PopupType::Settings(SettingsMode::ViewRoots))
    }

    pub(crate) fn settings_scroll_up(&mut self) {
        let roots_count = self.ui.get_roots().len();
        if roots_count > 0 {
            let current = self.ui.popup.selection.selected().unwrap_or(0);
            let new_selection = if current > 0 {
                current - 1
            } else {
                roots_count - 1 // Wrap to bottom
            };
            self.ui.popup.selection.select(Some(new_selection));
        }
    }

    pub(crate) fn settings_scroll_down(&mut self) {
        let roots_count = self.ui.get_roots().len();
        if roots_count > 0 {
            let current = self.ui.popup.selection.selected().unwrap_or(0);
            let new_selection = (current + 1) % roots_count; // Wrap to top
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
                let path = self.ui.popup.input.lines();
                let path = path[0].clone();
                if !path.is_empty() {
                    if let Err(e) = self.ui.add_root(&path) {
                        self.ui.set_error(e);
                    } else {
                        self.ui
                            .show_popup(PopupType::Settings(SettingsMode::ViewRoots));
                        self.update_library()?;
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
