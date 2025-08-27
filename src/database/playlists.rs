use indexmap::IndexMap;

use crate::{
    database::queries::{
        ADD_SONG_TO_PLAYLIST, CREATE_NEW_PLAYLIST, DELETE_PLAYLIST, GET_PLAYLISTS,
        PLAYLIST_BUILDER, REMOVE_SONG_FROM_PLAYLIST, UPDATE_PLAYLIST,
    },
    domain::Playlist,
    Database,
};
use anyhow::Result;
use rusqlite::params;

impl Database {
    pub fn create_playlist(&mut self, name: &str) -> Result<()> {
        self.conn.execute(CREATE_NEW_PLAYLIST, params![name])?;

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
        let tx = self.conn.transaction()?;

        tx.execute(
            ADD_SONG_TO_PLAYLIST,
            params![song_id.to_le_bytes(), playlist_id],
        )?;
        tx.execute(UPDATE_PLAYLIST, params![playlist_id])?;

        tx.commit()?;

        Ok(())
    }

    pub fn remove_from_playlist(&mut self, ps_id: i64) -> Result<()> {
        self.conn
            .execute(REMOVE_SONG_FROM_PLAYLIST, params![ps_id])?;

        Ok(())
    }

    pub fn build_playlists(&mut self) -> Result<IndexMap<(i64, String), Vec<(i64, u64)>>> {
        let mut stmt = self.conn.prepare(PLAYLIST_BUILDER)?;

        let rows = stmt.query_map([], |r| {
            let ps_id: Option<i64> = r.get("id")?;
            let name: String = r.get("name")?;
            let playlist_id: i64 = r.get("playlist_id")?;

            let song_id: Option<u64> = match r.get::<_, Option<Vec<u8>>>("song_id")? {
                Some(hash_bytes) => {
                    let hash_array: [u8; 8] = hash_bytes.try_into().map_err(|_| {
                        rusqlite::Error::InvalidColumnType(
                            2,
                            "song_id".to_string(),
                            rusqlite::types::Type::Blob,
                        )
                    })?;
                    Some(u64::from_le_bytes(hash_array))
                }
                None => None,
            };

            Ok((playlist_id, song_id, ps_id, name))
        })?;

        let mut playlist_map: IndexMap<(i64, String), Vec<(i64, u64)>> = IndexMap::new();

        for row in rows {
            let (playlist_id, song_id_opt, ps_id_opt, name) = row?;

            let entry = playlist_map
                .entry((playlist_id, name))
                .or_insert_with(Vec::new);

            if let (Some(song_id), Some(ps_id)) = (song_id_opt, ps_id_opt) {
                entry.push((ps_id, song_id))
            }
        }

        Ok(playlist_map)
    }
}
