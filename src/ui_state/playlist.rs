use crate::{
    library::{Playlist, PlaylistSong},
    ui_state::{LibraryView, PopupType, UiState},
};
use anyhow::{Result, anyhow, bail};

#[derive(PartialEq, Clone)]
pub enum PlaylistAction {
    Create,
    AddSong,
    Delete,
    Rename,
    CreateWithSongs,
}

impl UiState {
    pub fn get_playlists(&mut self) -> Result<()> {
        let playlist_db = self.db_worker.build_playlists()?;
        let songs_map = self.library.get_songs_map();

        self.playlists = playlist_db
            .iter()
            .map(|((id, name), track_ids)| {
                let tracklist = track_ids
                    .iter()
                    .filter_map(|&s_id| {
                        let ps_id = s_id.0;
                        let simple_song = songs_map.get(&s_id.1)?.clone();

                        Some(PlaylistSong {
                            id: ps_id,
                            song: simple_song,
                        })
                    })
                    .collect::<Vec<PlaylistSong>>();

                Playlist::new(*id, name.to_string(), tracklist)
            })
            .collect();

        Ok(())
    }

    pub fn create_playlist_popup(&mut self) {
        if self.get_sidebar_view() == &LibraryView::Playlists {
            self.show_popup(PopupType::Playlist(PlaylistAction::Create));
        }
    }

    pub fn create_playlist(&mut self) -> Result<()> {
        let name = self.get_popup_string();

        if name.is_empty() {
            bail!("Playlist name cannot be empty!");
        }

        if self
            .playlists
            .iter()
            .any(|p| p.name.to_lowercase() == name.to_lowercase())
        {
            bail!("Playlist name already exists!");
        }

        self.db_worker.create_playlist(name)?;

        self.get_playlists()?;

        if !self.playlists.is_empty() {
            self.display_state.playlist_pos.select_first();
        }

        self.set_legal_songs();
        self.close_popup();
        Ok(())
    }

    pub fn rename_playlist_popup(&mut self) {
        if self.get_selected_playlist().is_some() {
            self.show_popup(PopupType::Playlist(PlaylistAction::Rename));
        }
    }

    pub fn rename_playlist(&mut self) -> Result<()> {
        let playlist = self
            .get_selected_playlist()
            .ok_or_else(|| anyhow!("No playlist selected!"))?;

        let new_name = self.get_popup_string();

        if new_name.is_empty() {
            bail!("Playlist name cannot be empty!");
        }

        if self
            .playlists
            .iter()
            .filter(|p| p.id != playlist.id)
            .any(|p| p.name.to_lowercase() == new_name.to_lowercase())
        {
            bail!("Playlist name already exists!");
        }

        self.db_worker.rename_playlist(playlist.id, new_name)?;

        self.get_playlists()?;

        if !self.playlists.is_empty() {
            self.display_state.playlist_pos.select_first();
        }

        self.close_popup();
        Ok(())
    }

    pub fn delete_playlist_popup(&mut self) {
        if self.get_selected_playlist().is_some() {
            self.show_popup(PopupType::Playlist(PlaylistAction::Delete))
        }
    }

    pub fn delete_playlist(&mut self) -> Result<()> {
        let current_playlist = self.display_state.playlist_pos.selected();
        // let playlist_len =

        if let Some(idx) = current_playlist {
            let playlist_id = self.playlists[idx].id;
            self.db_worker.delete_playlist(playlist_id)?;

            self.get_playlists()?;
            self.set_legal_songs();
        }

        if self.playlists.is_empty() {
            self.display_state.playlist_pos.select(None);
            self.legal_songs.clear();
        }

        self.close_popup();

        Ok(())
    }

    pub fn add_to_playlist_popup(&mut self) {
        if self.legal_songs.len() == 0 {
            return;
        }
        self.popup.selection.select_first();
        self.show_popup(PopupType::Playlist(PlaylistAction::AddSong));
    }

    pub fn add_to_playlist(&mut self) -> Result<()> {
        match self.popup.selection.selected() {
            Some(playlist_idx) => {
                let playlist_id = self.playlists.get(playlist_idx).unwrap().id;
                match self.multi_select_empty() {
                    true => {
                        let song_id = self.get_selected_song()?.id;

                        self.db_worker.add_to_playlist(song_id, playlist_id)?;
                    }
                    false => {
                        let song_ids = self
                            .get_multi_select_songs()
                            .iter()
                            .map(|s| s.id)
                            .collect::<Vec<_>>();

                        self.db_worker
                            .add_to_playlist_multi(song_ids, playlist_id)?;
                        self.clear_multi_select();
                    }
                }
                self.close_popup()
            }
            None => bail!("Could not add to playlist"),
        };

        self.get_playlists()?;
        self.set_legal_songs();

        Ok(())
    }

    pub fn create_playlist_with_songs_popup(&mut self) {
        self.show_popup(PopupType::Playlist(PlaylistAction::CreateWithSongs));
    }

    pub fn create_playlist_with_songs(&mut self) -> Result<()> {
        let name = self.get_popup_string();

        if name.is_empty() {
            bail!("Playlist name cannot be empty!");
        }

        if self
            .playlists
            .iter()
            .any(|p| p.name.to_lowercase() == name.to_lowercase())
        {
            bail!("Playlist name already exists!");
        }

        self.db_worker.create_playlist(name)?;
        self.get_playlists()?;

        if let Some(new_playlist) = self.playlists.first() {
            let playlist_id = new_playlist.id;

            if !self.multi_select_empty() {
                let song_ids = self
                    .get_multi_select_songs()
                    .iter()
                    .map(|s| s.id)
                    .collect::<Vec<_>>();
                self.db_worker
                    .add_to_playlist_multi(song_ids, playlist_id)?;
                self.clear_multi_select();
            } else if let Ok(song) = self.get_selected_song() {
                self.db_worker.add_to_playlist(song.id, playlist_id)?;
            }

            self.get_playlists()?;
        }

        self.set_legal_songs();
        self.close_popup();
        Ok(())
    }
}
