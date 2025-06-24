use anyhow::anyhow;
use anyhow::Result;
use ratatui::widgets::ListState;
use std::sync::Arc;
use tui_textarea::TextArea;

use crate::ui_state::Pane;
use crate::{
    ui_state::{new_textarea, UiState},
    Library,
};

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
