use crate::{
    database::queries::{CREATE_NEW_PLAYLIST, GET_PLAYLISTS, UPDATE_PLAYLIST},
    domain::Playlist,
    Database,
};
use anyhow::Result;
use rusqlite::params;

impl Database {
    pub fn create_new_playlist(&mut self, name: &str) -> Result<()> {
        self.conn.execute(CREATE_NEW_PLAYLIST, params![name])?;

        Ok(())
    }

    pub fn update_playlist(&mut self, id: u64) -> Result<()> {
        self.conn.execute(UPDATE_PLAYLIST, params![id])?;

        Ok(())
    }

    pub fn get_playlists(&mut self) -> Result<Vec<Playlist>> {
        let mut stmt = self.conn.prepare(GET_PLAYLISTS)?;

        let rows = stmt.query_map([], |r| {
            let id: u64 = r.get("id")?;
            let name: String = r.get("name")?;

            Ok(Playlist::new(id, name))
        })?;

        let mut playlists = vec![];
        for row in rows {
            if let Ok(playlist) = row {
                playlists.push(playlist);
            }
        }

        Ok(playlists)
    }
}
