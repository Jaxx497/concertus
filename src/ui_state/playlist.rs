use crate::{domain::Playlist, ui_state::UiState};
use anyhow::{anyhow, Result};

#[derive(PartialEq)]
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
                let tracks = track_ids
                    .iter()
                    .filter_map(|&s_id| songs_map.get(&s_id).cloned())
                    .collect();

                Playlist {
                    id: *id,
                    name: name.to_string(),
                    tracks,
                }
            })
            .collect();

        Ok(())
    }

    pub fn create_playlist(&mut self, name: &str) -> Result<()> {
        if name.trim().is_empty() {
            return Err(anyhow!("Playlist name cannot be empty!"));
        }

        {
            let db = self.library.get_db();
            let mut db_lock = db.lock().unwrap();
            db_lock.create_playlist(name)?;
        }
        self.get_playlists()?;

        if self.display_state.playlist_pos.selected() == None {
            self.display_state.playlist_pos.select_first();
        }

        Ok(())
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

            // self.playlists = db_lock.get_playlists()?;
            self.get_playlists()?;
        }

        Ok(())
    }

    pub fn add_to_playlist(&mut self) -> Result<()> {
        if let Some(playlist_idx) = self.popup.selection.selected() {
            let song_id = self.get_selected_song()?.id;
            let playlist_id = self.playlists.get(playlist_idx).unwrap().id;

            let db = self.library.get_db();
            let mut db_lock = db.lock().unwrap();
            db_lock.add_to_playlist(song_id, playlist_id)?;

            self.close_popup()
        } else {
            return Err(anyhow!("Could not add to playlist"));
        };

        self.get_playlists()?;

        Ok(())
    }
}
