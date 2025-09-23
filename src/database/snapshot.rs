use anyhow::Result;
use rusqlite::params;

use crate::{
    database::queries::{GET_UI_SNAPSHOT, SET_SESSION_STATE},
    ui_state::UiSnapshot,
    Database,
};

impl Database {
    pub fn save_ui_snapshot(&mut self, snapshot: UiSnapshot) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare(SET_SESSION_STATE)?;
            for (key, value) in snapshot.to_pairs() {
                stmt.execute(params![key, value])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn load_ui_snapshot(&mut self) -> Result<Option<UiSnapshot>> {
        let mut stmt = self.conn.prepare(GET_UI_SNAPSHOT)?;

        let values: Vec<(String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(Result::ok)
            .collect();

        match values.is_empty() {
            true => Ok(None),
            false => Ok(Some(UiSnapshot::from_values(values))),
        }
    }
}
