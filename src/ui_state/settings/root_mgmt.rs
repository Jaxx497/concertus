use crate::{
    Library,
    app_core::Concertus,
    ui_state::{PopupType, SettingsMode, UiState},
};
use anyhow::{Result, bail};
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
        self.library = Arc::new(lib);

        Ok(())
    }

    pub fn remove_root(&mut self) -> Result<()> {
        if let Some(selected) = self.popup.selection.selected() {
            let roots = self.get_roots();
            if selected >= roots.len() {
                bail!("Invalid root index!");
            }

            let mut lib = Library::init();

            let bad_root = &roots[selected];
            lib.delete_root(&bad_root)?;

            self.library = Arc::new(lib);
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
                    self.ui.close_popup();
                }
            }
            _ => {}
        }
        Ok(())
    }
}
