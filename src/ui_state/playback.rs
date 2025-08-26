use super::{Mode, UiState};
use crate::{
    domain::{QueueSong, SimpleSong},
    player::{PlaybackState, PlayerState},
    strip_win_prefix,
    ui_state::LibraryView,
};
use anyhow::{anyhow, Context, Result};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};

const HISTORY_CAPACITY: usize = 50;
pub struct PlaybackCoordinator {
    pub queue: VecDeque<Arc<QueueSong>>,
    pub history: VecDeque<Arc<SimpleSong>>,
    pub waveform: Vec<f32>,
    pub(self) player_state: Arc<Mutex<PlayerState>>,
}

impl PlaybackCoordinator {
    pub fn new(player_state: Arc<Mutex<PlayerState>>) -> Self {
        PlaybackCoordinator {
            queue: VecDeque::new(),
            history: VecDeque::new(),
            waveform: Vec::new(),
            player_state,
        }
    }
}

// ===================
//   QUEUE & HISTORY
// =================
impl UiState {
    pub fn queue_is_empty(&self) -> bool {
        self.playback.queue.is_empty()
    }

    pub(crate) fn queue_song(&mut self, song: Option<Arc<SimpleSong>>) -> Result<()> {
        let simple_song = match song {
            Some(s) => s,
            None => self.get_selected_song()?,
        };

        let queue_song = self.make_playable_song(&simple_song)?;
        self.playback.queue.push_back(queue_song);
        Ok(())
    }

    pub fn queue_entity(&mut self) -> Result<()> {
        let songs = match self.get_mode() {
            Mode::Library(LibraryView::Albums) => {
                let album_idx = self
                    .display_state
                    .album_pos
                    .selected()
                    .ok_or_else(|| anyhow!("Illegal album selection!"))?;

                self.albums[album_idx].tracklist.clone()
            }
            Mode::Library(LibraryView::Playlists) => {
                let playlist_idx = self
                    .display_state
                    .playlist_pos
                    .selected()
                    .ok_or_else(|| anyhow!("Illegal playlist selection!"))?;

                self.playlists[playlist_idx].get_tracks()
            }
            _ => return Ok(()),
        };

        for song in songs {
            self.queue_song(Some(song))?;
        }
        Ok(())
    }

    pub(crate) fn add_to_history(&mut self, song: Arc<SimpleSong>) {
        if let Some(last) = self.playback.history.front() {
            if last.id == song.id {
                return;
            }
        }

        self.playback.history.push_front(song);
        while self.playback.history.len() > HISTORY_CAPACITY {
            self.playback.history.pop_back();
        }
    }

    pub(crate) fn load_history(&mut self) {
        self.playback.history = self
            .library
            .load_history(&self.library.get_songs_map())
            .unwrap_or_default();
    }

    pub fn peek_queue(&self) -> Option<&Arc<SimpleSong>> {
        self.playback.queue.front().map(|q| &q.meta)
    }

    pub fn get_prev_song(&mut self) -> Option<Arc<SimpleSong>> {
        match self.get_now_playing() {
            Some(_) => self.playback.history.remove(1),
            None => self.playback.history.remove(0),
        }
    }

    pub fn remove_song(&mut self) -> Result<()> {
        match *self.get_mode() {
            Mode::Library(LibraryView::Playlists) => {
                let song_idx = self
                    .display_state
                    .table_pos
                    .selected()
                    .ok_or_else(|| anyhow!("No song selected"))?;

                let playlist_id = self
                    .get_selected_playlist()
                    .ok_or_else(|| anyhow!("No playlist selected"))?
                    .id;

                let playlist = self
                    .playlists
                    .iter_mut()
                    .find(|p| p.id == playlist_id)
                    .ok_or_else(|| anyhow!("Playlist not found"))?;

                let ps_id = playlist
                    .tracklist
                    .get(song_idx)
                    .ok_or_else(|| anyhow!("Invalid song selection"))?
                    .id;

                self.library
                    .get_db()
                    .lock()
                    .map_err(|_| anyhow!("Failed to acquire database lock"))?
                    .remove_from_playlist(ps_id)?;

                playlist.tracklist.remove(song_idx);
            }
            Mode::Queue => {
                self.display_state
                    .table_pos
                    .selected()
                    .and_then(|idx| self.playback.queue.remove(idx));
            }
            _ => (),
        };

        self.set_legal_songs();
        Ok(())
    }
}

// ===============
//   PlayerState
// =============
impl UiState {
    pub fn update_player_state(&mut self, player_state: Arc<Mutex<PlayerState>>) {
        self.playback.player_state = player_state;
        self.check_player_error();
    }

    pub(crate) fn is_paused(&self) -> bool {
        let state = self.playback.player_state.lock().unwrap();
        state.state == PlaybackState::Paused
    }

    pub fn get_now_playing(&self) -> Option<Arc<SimpleSong>> {
        let state = self.playback.player_state.lock().unwrap();
        state.now_playing.clone()
    }

    pub fn get_playback_elapsed(&self) -> Duration {
        let state = self.playback.player_state.lock().unwrap();
        state.elapsed
    }

    pub fn is_not_playing(&self) -> bool {
        let state = self.playback.player_state.lock().unwrap();
        state.state == PlaybackState::Stopped
    }

    pub fn make_playable_song(&mut self, song: &Arc<SimpleSong>) -> Result<Arc<QueueSong>> {
        let path = self
            .library
            .get_path(song.id)
            .context("Could not retrieve path from database!")?;

        std::fs::metadata(&path).context(anyhow!(
            "Invalid file path!\n\nUnable to find: \"{}\"",
            strip_win_prefix(&path)
        ))?;

        Ok(Arc::new(QueueSong {
            meta: Arc::clone(&song),
            path,
        }))
    }
}

// ============
//   WAVEFORM
// ==========
impl UiState {
    pub fn get_waveform(&self) -> &[f32] {
        self.playback.waveform.as_slice()
    }

    pub fn set_waveform(&mut self, wf: Vec<f32>) {
        self.playback.waveform = wf
    }

    pub fn clear_waveform(&mut self) {
        self.playback.waveform.clear();
    }

    fn check_player_error(&mut self) {
        let error = self
            .playback
            .player_state
            .lock()
            .unwrap()
            .player_error
            .take();

        if let Some(e) = error {
            self.set_error(e);
        }
    }
}
