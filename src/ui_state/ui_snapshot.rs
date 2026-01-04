use anyhow::Result;

use crate::ui_state::ProgressDisplay;

use super::{AlbumSort, Mode, Pane, UiState};

#[derive(Default)]
pub struct UiSnapshot {
    pub mode: String,
    pub pane: String,
    pub album_sort: String,
    pub sidebar_percentage: u16,

    pub theme_name: String,

    pub song_selection: Option<usize>,
    pub album_selection: Option<usize>,
    pub playlist_selection: Option<usize>,

    pub song_sel_offset: usize,
    pub album_sel_offset: usize,
    pub playlist_sel_offset: usize,

    pub progress_display: String,
    pub smoothing_factor: f32,
}

impl UiSnapshot {
    pub fn to_pairs(&self) -> Vec<(&'static str, String)> {
        let mut pairs = vec![
            ("ui_mode", self.mode.clone()),
            ("ui_pane", self.pane.clone()),
            ("ui_album_sort", self.album_sort.clone()),
            ("ui_theme", self.theme_name.clone()),
            ("ui_smooth", format!("{:.1}", self.smoothing_factor)),
            ("ui_sidebar_percent", self.sidebar_percentage.to_string()),
            ("ui_progress_display", self.progress_display.to_string()),
        ];

        if let Some(pos) = self.album_selection {
            pairs.push(("ui_album_pos", pos.to_string()));
            pairs.push(("ui_album_offset", self.album_sel_offset.to_string()))
        }

        if let Some(pos) = self.playlist_selection {
            pairs.push(("ui_playlist_pos", pos.to_string()));
            pairs.push(("ui_playlist_offset", self.playlist_sel_offset.to_string()))
        }

        if let Some(pos) = self.song_selection {
            pairs.push(("ui_song_pos", pos.to_string()));
            pairs.push(("ui_song_offset", self.song_sel_offset.to_string()))
        }

        pairs
    }

    pub fn from_values(values: Vec<(String, String)>) -> Self {
        let mut snapshot = UiSnapshot::default();

        for (key, value) in values {
            match key.as_str() {
                "ui_mode" => snapshot.mode = value,
                "ui_pane" => snapshot.pane = value,
                "ui_progress_display" => snapshot.progress_display = value,
                "ui_theme" => snapshot.theme_name = value,
                "ui_album_sort" => snapshot.album_sort = value,
                "ui_album_pos" => snapshot.album_selection = value.parse().ok(),
                "ui_playlist_pos" => snapshot.playlist_selection = value.parse().ok(),
                "ui_album_offset" => snapshot.album_sel_offset = value.parse().unwrap_or(0),
                "ui_playlist_offset" => snapshot.playlist_sel_offset = value.parse().unwrap_or(0),
                "ui_song_pos" => snapshot.song_selection = value.parse().ok(),
                "ui_song_offset" => snapshot.song_sel_offset = value.parse::<usize>().unwrap_or(0),
                "ui_smooth" => snapshot.smoothing_factor = value.parse::<f32>().unwrap_or(1.0),
                "ui_sidebar_percent" => {
                    snapshot.sidebar_percentage = value.parse::<u16>().unwrap_or(30)
                }
                _ => {}
            }
        }

        snapshot
    }
}

impl UiState {
    pub fn create_snapshot(&self) -> UiSnapshot {
        let orig_pane = self.get_pane();
        let pane = match orig_pane {
            Pane::Popup => &self.popup.cached,
            _ => orig_pane,
        };

        UiSnapshot {
            mode: self.get_mode().to_string(),
            pane: pane.to_string(),
            album_sort: self.display_state.album_sort.to_string(),
            sidebar_percentage: self.display_state.sidebar_percent,

            theme_name: self.theme_manager.active.name.to_owned(),

            song_selection: self.display_state.table_pos.selected(),
            album_selection: self.display_state.album_pos.selected(),
            playlist_selection: self.display_state.playlist_pos.selected(),

            song_sel_offset: self.display_state.table_pos.offset(),
            album_sel_offset: self.display_state.album_pos.offset(),
            playlist_sel_offset: self.display_state.playlist_pos.offset(),

            progress_display: self.get_progress_display().to_string(),
            smoothing_factor: self.get_smoothing_factor(),
        }
    }

    pub fn save_state(&self) -> Result<()> {
        let snapshot = self.create_snapshot();
        self.db_worker.save_ui_snapshot(snapshot)?;
        Ok(())
    }

    pub fn restore_state(&mut self) -> Result<()> {
        // The order of these function calls is particularly important
        if let Some(snapshot) = self.db_worker.load_ui_snapshot()? {
            self.display_state.album_sort = AlbumSort::from_str(&snapshot.album_sort);

            self.sort_albums();

            if !snapshot.theme_name.is_empty() {
                if let Some(theme) = self.theme_manager.find_theme_by_name(&snapshot.theme_name) {
                    self.theme_manager.set_theme(theme.clone());
                }
            }

            if let Some(pos) = snapshot.album_selection {
                if pos < self.albums.len() {
                    self.display_state.album_pos.select(Some(pos));
                    *self.display_state.album_pos.offset_mut() = snapshot.album_sel_offset
                }
            }

            if let Some(pos) = snapshot.playlist_selection {
                if pos < self.playlists.len() {
                    self.display_state.playlist_pos.select(Some(pos));
                    *self.display_state.playlist_pos.offset_mut() = snapshot.playlist_sel_offset
                }
            }

            // Do not restore to queue or search mode
            let mode_to_restore = match snapshot.mode.as_str() {
                "search" | "queue" => "library_album",
                _ => &snapshot.mode,
            };

            let pane_to_restore = match snapshot.pane.as_str() {
                "search" => "tracklist",
                _ => &snapshot.pane,
            };

            self.set_mode(Mode::from_str(mode_to_restore));
            self.set_pane(Pane::from_str(pane_to_restore));

            self.set_smoothing_factor(snapshot.smoothing_factor);

            self.set_progress_display(ProgressDisplay::from_str(&snapshot.progress_display));

            self.display_state.sidebar_percent = snapshot.sidebar_percentage;

            if let Some(pos) = snapshot.song_selection {
                if pos < self.legal_songs.len() {
                    self.display_state.table_pos.select(Some(pos));
                    *self.display_state.table_pos.offset_mut() = snapshot.song_sel_offset
                }
            }
        }

        Ok(())
    }
}
