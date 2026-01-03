use crate::{
    domain::SimpleSong,
    key_handler::{Director, MoveDirection},
    ui_state::{LibraryView, Mode, Pane, UiState},
};
use anyhow::{anyhow, Result};
use indexmap::IndexSet;
use rand::seq::SliceRandom;
use std::sync::Arc;

impl UiState {
    pub fn get_multi_select_indices(&self) -> &IndexSet<usize> {
        &self.display_state.multi_select
    }

    pub fn toggle_multi_selection(&mut self) -> Result<()> {
        let song_idx = self.get_selected_idx()?;

        match self.display_state.multi_select.contains(&song_idx) {
            true => self.display_state.multi_select.swap_remove(&song_idx),
            false => self.display_state.multi_select.insert(song_idx),
        };

        Ok(())
    }

    pub fn multi_select_all(&mut self) -> Result<()> {
        if let Mode::Queue | Mode::Library(_) = self.get_mode() {
            let all_selected =
                (0..self.legal_songs.len()).all(|i| self.display_state.multi_select.contains(&i));

            match all_selected {
                true => self.clear_multi_select(),
                false => {
                    self.display_state.multi_select = (0..self.legal_songs.len()).collect();
                }
            }
        }
        Ok(())
    }

    pub fn get_multi_select_songs(&self) -> Vec<Arc<SimpleSong>> {
        self.display_state
            .multi_select
            .iter()
            .filter_map(|&idx| self.legal_songs.get(idx))
            .map(Arc::clone)
            .collect()
    }

    pub fn multi_select_empty(&self) -> bool {
        self.display_state.multi_select.is_empty()
    }

    pub fn clear_multi_select(&mut self) {
        self.display_state.multi_select.clear();
    }

    pub(crate) fn shift_position(&mut self, direction: MoveDirection) -> Result<()> {
        match self.multi_select_empty() {
            true => self.shift_position_single(direction),
            false => self.shift_position_multi(direction),
        }
    }

    pub(crate) fn shift_position_single(&mut self, direction: MoveDirection) -> Result<()> {
        let Some(display_idx) = self.display_state.table_pos.selected() else {
            return Ok(());
        };

        match self.get_mode() {
            Mode::Queue => {
                let target_idx = match direction {
                    MoveDirection::Up if display_idx > 0 => display_idx - 1,
                    MoveDirection::Down if display_idx < self.playback.queue.len() - 1 => {
                        display_idx + 1
                    }
                    _ => return Ok(()),
                };

                self.playback.queue.swap(display_idx, target_idx);
                self.scroll(match direction {
                    MoveDirection::Up => Director::Up(1),
                    MoveDirection::Down => Director::Down(1),
                });
            }

            Mode::Library(LibraryView::Playlists) => {
                let Some(playlist_idx) = self.display_state.playlist_pos.selected() else {
                    return Ok(());
                };

                let playlist = &mut self.playlists[playlist_idx];

                let target_idx = match direction {
                    MoveDirection::Up if display_idx > 0 => display_idx - 1,
                    MoveDirection::Down if display_idx < playlist.len() - 1 => display_idx + 1,
                    _ => return Ok(()),
                };

                let ps_id1 = playlist.tracklist[display_idx].id;
                let ps_id2 = playlist.tracklist[target_idx].id;

                self.db_worker.swap_position(ps_id1, ps_id2, playlist.id)?;
                playlist.tracklist.swap(display_idx, target_idx);

                self.scroll(match direction {
                    MoveDirection::Up => Director::Up(1),
                    MoveDirection::Down => Director::Down(1),
                });
            }
            _ => (),
        }
        self.set_legal_songs();

        Ok(())
    }

    pub(crate) fn shift_position_multi(&mut self, direction: MoveDirection) -> Result<()> {
        let mut indices = self
            .get_multi_select_indices()
            .iter()
            .copied()
            .collect::<Vec<_>>();

        indices.sort_unstable();
        let last_selected_idx = indices[indices.len() - 1];

        match self.get_mode() {
            Mode::Queue => {
                let queue_len = self.playback.queue.len();

                match direction {
                    MoveDirection::Up if indices[0] > 0 => {
                        indices.iter_mut().for_each(|idx| {
                            self.playback.queue.swap(*idx, *idx - 1);
                            *idx -= 1;
                        });
                    }
                    MoveDirection::Down if last_selected_idx < (queue_len - 1) => {
                        indices.iter_mut().rev().for_each(|idx| {
                            self.playback.queue.swap(*idx, *idx + 1);
                            *idx += 1;
                        });
                    }
                    _ => return Ok(()),
                }
            }
            Mode::Library(LibraryView::Playlists) => {
                let Some(playlist_idx) = self.display_state.playlist_pos.selected() else {
                    return Ok(());
                };

                let playlist = &mut self.playlists[playlist_idx];
                let playlist_len = playlist.tracklist.len();

                match direction {
                    MoveDirection::Up if indices[0] > 0 => {
                        for idx in indices.iter_mut() {
                            let ps_id1 = playlist.tracklist[*idx].id;
                            let ps_id2 = playlist.tracklist[*idx - 1].id;
                            self.db_worker.swap_position(ps_id1, ps_id2, playlist.id)?;
                            playlist.tracklist.swap(*idx, *idx - 1);
                            *idx -= 1;
                        }
                    }
                    MoveDirection::Down if last_selected_idx < (playlist_len - 1) => {
                        for idx in indices.iter_mut().rev() {
                            let ps_id1 = playlist.tracklist[*idx].id;
                            let ps_id2 = playlist.tracklist[*idx + 1].id;
                            self.db_worker.swap_position(ps_id1, ps_id2, playlist.id)?;
                            playlist.tracklist.swap(*idx, *idx + 1);
                            *idx += 1;
                        }
                    }
                    _ => return Ok(()),
                }
            }
            // Do nothing but maintain selection in other modes
            _ => return Ok(()),
        }
        self.set_legal_songs();
        self.display_state.multi_select = indices.iter().copied().collect::<IndexSet<_>>();

        Ok(())
    }

    pub fn add_to_queue_multi(&mut self, shuffle: bool) -> Result<()> {
        let mut songs = if !self.multi_select_empty() {
            self.get_multi_select_songs()
        } else if matches!(self.get_pane(), Pane::SideBar) {
            match self.get_mode() {
                Mode::Library(LibraryView::Albums) => {
                    let album_idx = self
                        .display_state
                        .album_pos
                        .selected()
                        .ok_or_else(|| anyhow!("Illegal album selection!"))?;

                    self.albums[album_idx].get_tracklist()
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
            }
        } else {
            return Ok(());
        };

        if shuffle {
            songs.shuffle(&mut rand::rng());
        }

        for song in songs {
            self.add_to_queue_single(Some(song))?;
        }
        self.clear_multi_select();
        Ok(())
    }

    pub fn remove_song_multi(&mut self) -> Result<()> {
        match *self.get_mode() {
            Mode::Library(LibraryView::Playlists) => {
                // Obtain selected playlist id
                let playlist_id = self
                    .get_selected_playlist()
                    .ok_or_else(|| anyhow!("No song selected"))?
                    .id;

                // Obtain playlist_song_ids that match the multi_select ids
                let ps_ids_to_remove = {
                    let playlist = self
                        .playlists
                        .iter()
                        .find(|p| p.id == playlist_id)
                        .ok_or_else(|| anyhow!("Playlist not found"))?;

                    self.get_multi_select_indices()
                        .iter()
                        .filter_map(|&idx| playlist.tracklist.get(idx).map(|ps| ps.id))
                        .collect()
                };

                self.db_worker.remove_from_playlist(ps_ids_to_remove)?;

                // Create a sorted list of indicies
                let mut indicies = self.get_multi_select_indices().clone();
                indicies.sort_unstable();

                // Declare after indicies declaration to avoid fighting with borrow checker
                let playlist = self
                    .playlists
                    .iter_mut()
                    .find(|p| p.id == playlist_id)
                    .ok_or_else(|| anyhow!("Playlist not found"))?;

                // Remove indicies in reverse order
                for &idx in indicies.iter().rev() {
                    if idx < playlist.len() {
                        playlist.tracklist.remove(idx);
                    }
                }
            }
            Mode::Queue => {
                let mut indicies = self.get_multi_select_indices().clone();
                indicies.sort_unstable();

                // Remove indicies in reverse order
                for &idx in indicies.iter().rev() {
                    if idx < self.playback.queue.len() {
                        self.playback.remove_from_queue(idx);
                    }
                }
            }
            _ => (),
        }

        self.clear_multi_select();
        Ok(())
    }
}
