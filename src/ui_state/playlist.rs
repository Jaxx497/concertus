use crate::ui_state::UiState;
use anyhow::{anyhow, Result};

#[derive(PartialEq)]
pub enum PlaylistAction {
    Create,
    Delete,
    Rename,
}

impl UiState {
    pub fn create_playlist(&mut self, name: &str) -> Result<()> {
        if name.trim().is_empty() {
            return Err(anyhow!("Playlist name cannot be empty!"));
        }

        let db = self.library.get_db();
        let mut db_lock = db.lock().unwrap();
        db_lock.create_new_playlist(name)?;
        self.playlists = db_lock.get_playlists()?;
        drop(db_lock);

        // self.playlists = {
        //     let mut db_lock = db.lock().unwrap();
        //     db_lock.get_playlists()?
        // };

        Ok(())
    }
}
