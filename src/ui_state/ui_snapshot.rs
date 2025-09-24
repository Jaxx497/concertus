use anyhow::Result;

use crate::ui_state::ProgressDisplay;

use super::{AlbumSort, Mode, Pane, UiState};

#[derive(Default)]
pub struct UiSnapshot {
    pub mode: String,
    pub pane: String,
    pub album_sort: String,
    pub album_selection: Option<usize>,
    pub playlist_selection: Option<usize>,
    pub progress_display: String,
    pub song_selection: Option<usize>,
    pub smoothing_factor: f32,
    pub sidebar_percentage: u16,
}

impl UiSnapshot {
    pub fn to_pairs(&self) -> Vec<(&'static str, String)> {
        let mut pairs = vec![
            ("ui_mode", self.mode.clone()),
            ("ui_pane", self.pane.clone()),
            ("ui_album_sort", self.album_sort.clone()),
            ("ui_smooth", format!("{:.1}", self.smoothing_factor)),
            ("ui_sidebar_percent", format!("{}", self.sidebar_percentage)),
            ("ui_progress_display", self.progress_display.to_string()),
        ];

        if let Some(pos) = self.album_selection {
            pairs.push(("ui_album_pos", pos.to_string()));
        }

        if let Some(pos) = self.playlist_selection {
            pairs.push(("ui_playlist_pos", pos.to_string()));
        }

        if let Some(pos) = self.song_selection {
            pairs.push(("ui_song_pos", pos.to_string()));
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
                "ui_album_sort" => snapshot.album_sort = value,
                "ui_album_pos" => snapshot.album_selection = value.parse().ok(),
                "ui_playlist_pos" => snapshot.playlist_selection = value.parse().ok(),
                "ui_song_pos" => snapshot.song_selection = value.parse().ok(),
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
            album_selection: self.display_state.album_pos.selected(),
            playlist_selection: self.display_state.playlist_pos.selected(),
            progress_display: self.playback_view.progress_display.to_string(),
            song_selection: self.display_state.table_pos.selected(),
            smoothing_factor: self.playback_view.waveform_smoothing,
            sidebar_percentage: self.display_state.sidebar_percent,
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

            if let Some(pos) = snapshot.album_selection {
                if pos < self.albums.len() {
                    self.display_state.album_pos.select(Some(pos));
                }
            }

            if let Some(pos) = snapshot.playlist_selection {
                if pos < self.playlists.len() {
                    self.display_state.playlist_pos.select(Some(pos));
                }
            }

            // Do not restore to queue mode
            let mode_to_restore = match snapshot.mode.as_str() {
                "queue" => "library_album",
                _ => &snapshot.mode,
            };

            self.set_mode(Mode::from_str(mode_to_restore));
            self.set_pane(Pane::from_str(&snapshot.pane));

            self.playback_view.waveform_smoothing = snapshot.smoothing_factor;
            self.playback_view.progress_display =
                ProgressDisplay::from_str(&snapshot.progress_display);
            self.display_state.sidebar_percent = snapshot.sidebar_percentage;

            if let Some(pos) = snapshot.song_selection {
                if pos < self.legal_songs.len() {
                    self.display_state.table_pos.select(Some(pos));
                }
            }
        }

        Ok(())
    }
}
