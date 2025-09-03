use crate::{
    domain::{Playlist, PlaylistSong},
    ui_state::{LibraryView, PopupType, UiState},
};
use anyhow::{Result, anyhow};

#[derive(PartialEq, Clone)]
pub enum PlaylistAction {
    Create,
    AddSong,
    Delete,
    Rename,
}

impl UiState {
    pub fn get_playlists(&mut self) -> Result<()> {
        let playlist_db = {
            let db = self.library.get_db();
            let mut db_lock = db.lock().unwrap();
            db_lock.build_playlists()?
        };

        let songs_map = self.library.get_songs_map();

        self.playlists = playlist_db
            .iter()
            .map(|((id, name), track_ids)| {
                let tracklist = track_ids
                    .iter()
                    .filter_map(|&s_id| {
                        let ps_id = s_id.0;
                        let simple_song = songs_map.get(&s_id.1).unwrap().clone();

                        Some(PlaylistSong {
                            id: ps_id,
                            song: simple_song,
                        })
                    })
                    .collect();

                Playlist {
                    id: *id,
                    name: name.to_string(),
                    tracklist,
                }
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
            return Err(anyhow!("Playlist name cannot be empty!"));
        }

        if self
            .playlists
            .iter()
            .any(|p| p.name.to_lowercase() == name.to_lowercase())
        {
            return Err(anyhow!("Playlist name already exists!"));
        }

        {
            let db = self.library.get_db();
            let mut db_lock = db.lock().unwrap();
            db_lock.create_playlist(&name)?;
        }

        self.get_playlists()?;

        if self.display_state.playlist_pos.selected() == None {
            self.display_state.playlist_pos.select_first();
        }

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

        let name = self.get_popup_string();

        if name.is_empty() {
            return Err(anyhow!("Playlist name cannot be empty!"));
        }

        if self
            .playlists
            .iter()
            .filter(|p| p.id != playlist.id)
            .any(|p| p.name.to_lowercase() == name.to_lowercase())
        {
            return Err(anyhow!("Playlist name already exists!"));
        }

        {
            let db = self.library.get_db();
            let mut db_lock = db.lock().unwrap();
            db_lock.rename_playlist(&name, playlist.id)?;
        }

        self.get_playlists()?;
        self.display_state.playlist_pos.select_first();
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

        if let Some(idx) = current_playlist {
            let playlist_id = self.playlists[idx].id;
            {
                let db = self.library.get_db();
                let mut db_lock = db.lock().unwrap();
                db_lock.delete_playlist(playlist_id)?;
            }
            self.get_playlists()?;
            self.set_legal_songs();
        }

        self.close_popup();

        Ok(())
    }

    pub fn add_to_playlist_popup(&mut self) {
        self.popup.selection.select_first();
        self.show_popup(super::PopupType::Playlist(PlaylistAction::AddSong));
    }

    pub fn add_to_playlist(&mut self) -> Result<()> {
        if let Some(playlist_idx) = self.popup.selection.selected() {
            let playlist_id = self.playlists.get(playlist_idx).unwrap().id;

            match self.get_bulk_sel().is_empty() {
                true => {
                    let song_id = self.get_selected_song()?.id;

                    let db = self.library.get_db();
                    let mut db_lock = db.lock().unwrap();
                    db_lock.add_to_playlist(song_id, playlist_id)?;
                }
                false => {
                    let song_ids = self.get_bulk_sel().iter().map(|s| s.id).collect::<Vec<_>>();

                    let db = self.library.get_db();
                    let mut db_lock = db.lock().unwrap();

                    db_lock.add_to_playlist_bulk(song_ids, playlist_id)?;
                    self.clear_bulk_sel();
                }
            }

            self.close_popup()
        } else {
            return Err(anyhow!("Could not add to playlist"));
        };

        self.get_playlists()?;

        Ok(())
    }
}
