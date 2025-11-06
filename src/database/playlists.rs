use crate::{Database, database::queries::*};
use anyhow::Result;
use indexmap::IndexMap;
use rusqlite::params;

impl Database {
    pub fn create_playlist(&mut self, name: &str) -> Result<()> {
        self.conn.execute(CREATE_NEW_PLAYLIST, params![name])?;

        Ok(())
    }

    pub fn delete_playlist(&mut self, id: i64) -> Result<()> {
        self.conn.execute(DELETE_PLAYLIST, params![id])?;

        Ok(())
    }

    pub fn rename_playlist(&mut self, new_name: &str, playlist_id: i64) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            tx.execute(RENAME_PLAYLIST, params![new_name, playlist_id])?;
            tx.execute(UPDATE_PLAYLIST, params![playlist_id])?;
        }
        tx.commit()?;

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

    pub fn add_to_playlist_multi(&mut self, songs: Vec<u64>, playlist_id: i64) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let start_pos = tx
                .query_row(GET_PLAYLIST_POSITION_NEXT, params![playlist_id], |row| {
                    row.get(0)
                })
                .unwrap_or(0)
                + 1;

            let mut stmt = tx.prepare_cached(ADD_SONG_TO_PLAYLIST_WITH_POSITION)?;

            for (i, song) in songs.iter().enumerate() {
                stmt.execute(params![
                    song.to_le_bytes(),
                    playlist_id,
                    start_pos + i as i64
                ])?;
            }

            tx.execute(UPDATE_PLAYLIST, params![playlist_id])?;
        }
        tx.commit()?;

        Ok(())
    }

    pub fn remove_from_playlist(&mut self, ps_id: &[i64]) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare_cached(REMOVE_SONG_FROM_PLAYLIST)?;
            for id in ps_id {
                stmt.execute(params![id])?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    pub fn swap_position(&mut self, ps_id1: i64, ps_id2: i64, playlist_id: i64) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let pos1: i64 = tx.query_row(GET_PLAYLIST_POS, params![ps_id1], |row| row.get(0))?;

            let pos2: i64 = tx.query_row(GET_PLAYLIST_POS, params![ps_id2], |row| row.get(0))?;

            // Three-step swap to avoid unique constraint violation
            tx.execute(UPDATE_PLAYLIST_POS, params![-1, ps_id1])?;
            tx.execute(UPDATE_PLAYLIST_POS, params![pos1, ps_id2])?;
            tx.execute(UPDATE_PLAYLIST_POS, params![pos2, ps_id1])?;

            tx.execute(UPDATE_PLAYLIST, params![playlist_id])?;
        }

        tx.commit()?;

        Ok(())
    }

    pub fn build_playlists(&mut self) -> Result<IndexMap<(i64, String), Vec<(i64, u64)>>> {
        let mut stmt = self.conn.prepare_cached(PLAYLIST_BUILDER)?;

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
