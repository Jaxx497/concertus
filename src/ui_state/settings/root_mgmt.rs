use crate::{
    app_core::Concertus,
    ui_state::{new_textarea, Pane, UiState},
    Library,
};
use anyhow::{anyhow, Result};
use ratatui::widgets::ListState;
use std::sync::Arc;
use tui_textarea::TextArea;

#[derive(Default, PartialEq, Clone)]
pub enum SettingsMode {
    #[default]
    ViewRoots,
    AddRoot,
    RemoveRoot,
}

pub struct Settings {
    pub settings_mode: SettingsMode,
    pub settings_selection: ListState,
    pub root_input: TextArea<'static>,
}

impl Settings {
    pub fn new() -> Self {
        Settings {
            settings_mode: SettingsMode::default(),
            settings_selection: ListState::default().with_selected(Some(0)),
            root_input: new_textarea("Enter path to directory"),
        }
    }
}

impl UiState {
    pub fn get_settings_mode(&self) -> &SettingsMode {
        &self.settings.settings_mode
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
        if let Some(selected) = self.settings.settings_selection.selected() {
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
        self.settings.settings_mode = SettingsMode::ViewRoots;
        if !self.get_roots().is_empty() {
            self.settings.settings_selection.select(Some(0));
        }
        self.settings.root_input.select_all();
        self.settings.root_input.cut();
        self.set_pane(Pane::Popup);
    }
}

impl Concertus {
    pub(crate) fn settings_remove_root(&mut self) {
        if !self.ui.get_roots().is_empty() {
            self.ui.settings.settings_mode = SettingsMode::RemoveRoot;
        }
    }

    pub(crate) fn activate_settings(&mut self) {
        match self.ui.get_pane() {
            Pane::Popup => self.ui.soft_reset(),
            _ => {
                self.ui.set_pane(Pane::Popup);
                self.ui.settings.settings_mode = SettingsMode::ViewRoots
            }
        }
    }

    pub(crate) fn settings_scroll_up(&mut self) {
        let roots_count = self.ui.get_roots().len();
        if roots_count > 0 {
            let current = self.ui.settings.settings_selection.selected().unwrap_or(0);
            let new_selection = if current > 0 {
                current - 1
            } else {
                roots_count - 1 // Wrap to bottom
            };
            self.ui
                .settings
                .settings_selection
                .select(Some(new_selection));
        }
    }

    pub(crate) fn settings_scroll_down(&mut self) {
        let roots_count = self.ui.get_roots().len();
        if roots_count > 0 {
            let current = self.ui.settings.settings_selection.selected().unwrap_or(0);
            let new_selection = (current + 1) % roots_count; // Wrap to top
            self.ui
                .settings
                .settings_selection
                .select(Some(new_selection));
        }
    }

    pub(crate) fn settings_add_root(&mut self) {
        self.ui.settings.settings_mode = SettingsMode::AddRoot;
        self.ui.settings.root_input.select_all();
        self.ui.settings.root_input.cut();
    }

    pub(crate) fn settings_root_confirm(&mut self) -> anyhow::Result<()> {
        match self.ui.settings.settings_mode {
            SettingsMode::AddRoot => {
                let path = self.ui.settings.root_input.lines();
                let path = path[0].clone();
                if !path.is_empty() {
                    if let Err(e) = self.ui.add_root(&path) {
                        self.ui.set_error(e);
                    } else {
                        self.ui.settings.settings_mode = SettingsMode::ViewRoots;
                        self.update_library()?;
                    }
                }
            }
            SettingsMode::RemoveRoot => {
                if let Err(e) = self.ui.remove_root() {
                    self.ui.set_error(e);
                } else {
                    self.ui.settings.settings_mode = SettingsMode::ViewRoots;
                    self.ui.settings.settings_selection.select(Some(0));
                    self.update_library()?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
