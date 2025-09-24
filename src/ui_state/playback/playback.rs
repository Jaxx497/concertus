use crate::{
    domain::{QueueSong, SimpleSong, SongDatabase},
    player::{PlaybackState, PlayerState},
    strip_win_prefix,
    ui_state::{LibraryView, Mode, UiState},
};
use anyhow::{Context, Result, anyhow};
use std::{
    collections::{HashSet, VecDeque},
    sync::{Arc, Mutex},
    time::Duration,
};

const HISTORY_CAPACITY: usize = 50;

pub struct PlaybackCoordinator {
    pub queue: VecDeque<Arc<QueueSong>>,
    pub history: VecDeque<Arc<SimpleSong>>,
    pub player_state: Arc<Mutex<PlayerState>>,
}

impl PlaybackCoordinator {
    pub fn new(player_state: Arc<Mutex<PlayerState>>) -> Self {
        PlaybackCoordinator {
            queue: VecDeque::new(),
            history: VecDeque::new(),
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

    pub fn queue_song(&mut self, song: Option<Arc<SimpleSong>>) -> Result<()> {
        match self.bulk_select_empty() {
            true => self.add_to_queue_single(song),
            false => self.add_to_queue_bulk(),
        }?;

        self.set_legal_songs();
        Ok(())
    }

    pub(crate) fn add_to_queue_single(&mut self, song: Option<Arc<SimpleSong>>) -> Result<()> {
        let simple_song = match song {
            Some(s) => s,
            None => self.get_selected_song()?,
        };

        let queue_song = self.make_playable_song(&simple_song)?;
        self.playback.queue.push_back(queue_song);
        Ok(())
    }

    pub fn add_to_queue_bulk(&mut self) -> Result<()> {
        let songs;

        if !self.get_bulk_sel().is_empty() {
            songs = self
                .display_state
                .bulk_select
                .clone()
                .into_iter()
                .collect::<Vec<Arc<SimpleSong>>>();
            self.clear_bulk_sel();
        } else {
            songs = match self.get_mode() {
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
        }

        for song in songs {
            self.add_to_queue_single(Some(song))?;
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
        let song_map = self.library.get_songs_map().to_owned();
        self.playback.history = self.db_worker.import_history(song_map).unwrap_or_default();
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
        let selected_song = self.get_selected_song()?;

        let current_is_selected = self.get_bulk_sel().contains(&selected_song);

        match (self.bulk_select_empty(), current_is_selected) {
            (false, true) => self.remove_song_bulk()?,
            _ => self.remove_song_single()?,
        }

        self.set_legal_songs();
        Ok(())
    }

    pub fn remove_song_single(&mut self) -> Result<()> {
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

                self.db_worker.remove_from_playlist(vec![ps_id])?;

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
        Ok(())
    }

    pub fn remove_song_bulk(&mut self) -> Result<()> {
        match *self.get_mode() {
            Mode::Library(LibraryView::Playlists) => {
                let playlist_id = self
                    .get_selected_playlist()
                    .ok_or_else(|| anyhow!("No song selected"))?
                    .id;

                let removal_ids = self
                    .get_bulk_sel()
                    .iter()
                    .map(|s| s.id)
                    .collect::<HashSet<_>>();

                let ps_ids_to_remove = {
                    let playlist = self
                        .playlists
                        .iter_mut()
                        .find(|p| p.id == playlist_id)
                        .ok_or_else(|| anyhow!("Playlist not found"))?;

                    playlist
                        .tracklist
                        .iter()
                        .filter(|ps| removal_ids.contains(&ps.song.id))
                        .map(|ps| ps.id)
                        .collect::<Vec<_>>()
                };

                self.db_worker.remove_from_playlist(ps_ids_to_remove)?;

                let playlist = self
                    .playlists
                    .iter_mut()
                    .find(|p| p.id == playlist_id)
                    .ok_or_else(|| anyhow!("Playlist not found"))?;

                playlist
                    .tracklist
                    .retain(|playlist_song| !removal_ids.contains(&playlist_song.song.id));
            }
            Mode::Queue => {
                let removal_ids = self
                    .get_bulk_sel()
                    .iter()
                    .map(|s| s.id)
                    .collect::<HashSet<_>>();

                self.playback
                    .queue
                    .retain(|qs| !removal_ids.contains(&qs.meta.id));
            }
            _ => (),
        }

        self.clear_bulk_sel();
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

    pub fn set_playback_state(&mut self, playback: PlaybackState) {
        let mut state = self.playback.player_state.lock().unwrap();
        state.state = playback
    }

    pub fn get_playback_elapsed(&self) -> Duration {
        let state = self.playback.player_state.lock().unwrap();
        state.elapsed
    }

    pub fn is_not_playing(&self) -> bool {
        let state = self.playback.player_state.lock().unwrap();
        state.state == PlaybackState::Stopped
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

    pub fn make_playable_song(&mut self, song: &Arc<SimpleSong>) -> Result<Arc<QueueSong>> {
        let path = song.get_path()?;

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
