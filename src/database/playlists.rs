use crate::{
    database::queries::{
        ADD_SONG_TO_PLAYLIST, CREATE_NEW_PLAYLIST, DELETE_PLAYLIST, GET_PLAYLISTS, UPDATE_PLAYLIST,
    },
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

    pub fn update_playlist(&mut self, id: i64) -> Result<()> {
        self.conn.execute(UPDATE_PLAYLIST, params![id])?;

        Ok(())
    }

    pub fn get_playlists(&mut self) -> Result<Vec<Playlist>> {
        let mut stmt = self.conn.prepare(GET_PLAYLISTS)?;

        let rows = stmt.query_map([], |r| {
            let id: i64 = r.get("id")?;
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

    pub fn delete_playlist(&mut self, id: i64) -> Result<()> {
        self.conn.execute(DELETE_PLAYLIST, params![id])?;

        Ok(())
    }

    pub fn add_to_playlist(&mut self, song_id: u64, playlist_id: i64) -> Result<()> {
        self.conn.execute(
            ADD_SONG_TO_PLAYLIST,
            params![song_id.to_le_bytes(), playlist_id],
        )?;
        Ok(())
    }
}
