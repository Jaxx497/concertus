use crate::{
    database::{Database, DB_BOUND},
    domain::SimpleSong,
    ui_state::UiSnapshot,
    SongMap,
};
use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use std::{
    collections::{HashSet, VecDeque},
    sync::Arc,
    thread,
};

pub enum DbMessage {
    Operation(Box<dyn FnOnce(&mut Database) + Send>),
    Shutdown,
}

pub struct DbWorker {
    sender: crossbeam_channel::Sender<DbMessage>,
    pub handle: Option<thread::JoinHandle<()>>,
}

impl DbWorker {
    pub fn new() -> Result<Self> {
        let (sender, receiver) = crossbeam_channel::bounded::<DbMessage>(DB_BOUND);

        let handle = thread::spawn(move || {
            let mut db = match Database::open() {
                Ok(db) => db,
                Err(e) => {
                    eprintln!("Failed to open database in worker: {}", e);
                    return;
                }
            };

            while let Ok(msg) = receiver.recv() {
                match msg {
                    DbMessage::Operation(operation) => operation(&mut db),
                    DbMessage::Shutdown => break,
                }
            }
        });

        Ok(DbWorker {
            sender,
            handle: Some(handle),
        })
    }

    // Fire and forget operation
    pub fn execute<F>(&self, operation: F)
    where
        F: FnOnce(&mut Database) + Send + 'static,
    {
        let _ = self.sender.send(DbMessage::Operation(Box::new(operation)));
    }

    // Operations that need a response
    pub fn execute_sync<F, T>(&self, operation: F) -> Result<T>
    where
        F: FnOnce(&mut Database) -> Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let (result_tx, result_rx) = crossbeam_channel::bounded(128);

        self.execute(move |db| {
            let result = operation(db);
            let _ = result_tx.send(result);
        });

        result_rx
            .recv()
            .map_err(|_| anyhow::anyhow!("Worker dropped"))?
    }

    pub fn shutdown(&mut self) -> Result<()> {
        self.sender
            .send(DbMessage::Shutdown)
            .expect("Could not shutdown dbworker");

        if let Some(handle) = self.handle.take() {
            handle
                .join()
                .map_err(|_| anyhow!("Worker thread panicked!"))?;
        }

        Ok(())
    }
}

// Convenience functions
impl DbWorker {
    pub fn create_playlist(&self, name: String) -> Result<()> {
        self.execute_sync(move |db| db.create_playlist(&name))
    }

    pub fn delete_playlist(&self, id: i64) -> Result<()> {
        self.execute_sync(move |db| db.delete_playlist(id))
    }

    pub fn add_to_playlist(&self, song_id: u64, playlist_id: i64) -> Result<()> {
        self.execute_sync(move |db| db.add_to_playlist(song_id, playlist_id))
    }

    pub fn add_to_playlist_multi(&self, song_ids: Vec<u64>, playlist_id: i64) -> Result<()> {
        self.execute_sync(move |db| db.add_to_playlist_multi(song_ids, playlist_id))
    }

    pub fn rename_playlist(&self, id: i64, new_name: String) -> Result<()> {
        self.execute_sync(move |db| db.rename_playlist(&new_name, id))
    }

    pub fn remove_from_playlist(&self, ps_ids: Vec<i64>) -> Result<()> {
        self.execute_sync(move |db| db.remove_from_playlist(&ps_ids))
    }

    pub fn swap_position(&self, ps_id1: i64, ps_id2: i64, playlist_id: i64) -> Result<()> {
        self.execute_sync(move |db| db.swap_position(ps_id1, ps_id2, playlist_id))
    }

    pub fn get_hashes(&self) -> Result<HashSet<u64>> {
        self.execute_sync(move |db| db.get_hashes())
    }

    pub fn build_playlists(&mut self) -> Result<IndexMap<(i64, String), Vec<(i64, u64)>>> {
        self.execute_sync(move |db| db.build_playlists())
    }

    pub fn save_history(&self, history: Vec<u64>) -> Result<()> {
        self.execute_sync(move |db| db.save_history_to_db(&history))
    }

    pub fn save_ui_snapshot(&self, snapshot: UiSnapshot) -> Result<()> {
        self.execute_sync(move |db| db.save_ui_snapshot(snapshot))
    }

    pub fn load_ui_snapshot(&self) -> Result<Option<UiSnapshot>> {
        self.execute_sync(move |db| db.load_ui_snapshot())
    }

    pub fn get_all_songs(&self) -> Result<SongMap> {
        self.execute_sync(move |db| db.get_all_songs())
    }

    pub fn import_history(&self, song_map: SongMap) -> Result<VecDeque<Arc<SimpleSong>>> {
        self.execute_sync(move |db| db.import_history(&song_map))
    }

    pub fn save_history_to_db(&self, history: Vec<u64>) -> Result<()> {
        self.execute_sync(move |db| db.save_history_to_db(&history))
    }

    pub fn get_song_path(&self, id: u64) -> Result<String> {
        self.execute_sync(move |db| db.get_song_path(id))
    }

    pub fn update_play_count(&self, song_id: u64) {
        self.execute(move |db| {
            let _ = db.update_play_count(song_id);
        });
    }

    pub fn set_waveform(&self, song_id: u64, waveform: Vec<f32>) {
        self.execute(move |db| {
            let _ = db.set_waveform(song_id, &waveform);
        });
    }
}

impl Drop for DbWorker {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
