use crate::{
    domain::SimpleSong,
    key_handler::{Director, Incrementor, SelectionType},
    ui_state::{Mode, UiState},
};
use anyhow::{anyhow, Result};
use indexmap::IndexSet;
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

    pub fn get_songs_by_selection(
        &mut self,
        sel_type: SelectionType,
    ) -> Result<Vec<Arc<SimpleSong>>> {
        let selection = match sel_type {
            SelectionType::Multi => self.get_multi_select_songs(),
            SelectionType::Album => self
                .get_selected_album()
                .ok_or(anyhow!("Illegal album selection"))?
                .get_tracklist(),
            SelectionType::Playlist => self
                .get_selected_playlist()
                .ok_or(anyhow!("Illegal album selection"))?
                .get_tracklist(),
        };

        self.clear_multi_select();
        Ok(selection)
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

    pub fn remove_from_playlist(&mut self) -> Result<()> {
        let song_idx = self.get_selected_idx()?;

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
            .ok_or_else(|| anyhow!("Invalid song selection!"))?
            .id;

        self.db_worker.remove_from_playlist(vec![ps_id])?;

        playlist.tracklist.remove(song_idx);
        playlist.update_length();

        Ok(())
    }

    pub fn remove_from_playlist_multi(&mut self) -> Result<()> {
        // Obtain selected playlist id
        let playlist_id = self
            .get_selected_playlist()
            .ok_or_else(|| anyhow!("No song selected"))?
            .id;

        let mut indicies = self.get_multi_select_indices().clone();
        indicies.sort_unstable();

        // Obtain playlist_song_ids that match the multi_select ids
        let ps_ids = {
            let playlist = self
                .playlists
                .iter()
                .find(|p| p.id == playlist_id)
                .ok_or_else(|| anyhow!("Playlist not found"))?;

            indicies
                .iter()
                .filter_map(|&idx| playlist.tracklist.get(idx).map(|ps| ps.id))
                .collect()
        };

        self.db_worker.remove_from_playlist(ps_ids)?;

        // Redeclare to avoid fighting with borrow checker
        let playlist = self
            .playlists
            .iter_mut()
            .find(|p| p.id == playlist_id)
            .unwrap();

        // Remove indicies in reverse order
        for &idx in indicies.iter().rev() {
            if idx < playlist.len() {
                playlist.tracklist.remove(idx);
            }
        }

        playlist.update_length();
        self.clear_multi_select();
        Ok(())
    }

    pub(crate) fn update_multi_select(&mut self, indices: Vec<usize>) {
        self.display_state.multi_select = IndexSet::from_iter(indices.into_iter());
    }

    pub fn shift_playlist_position(&mut self, dir: Incrementor) -> Result<()> {
        match self.multi_select_empty() {
            true => self.shift_playlist_position_single(dir)?,
            false => self.shift_playlist_position_multi(dir)?,
        }

        Ok(())
    }

    fn shift_playlist_position_single(&mut self, direction: Incrementor) -> Result<()> {
        let display_idx = self.get_selected_idx()?;

        let Some(playlist_idx) = self.display_state.playlist_pos.selected() else {
            return Ok(());
        };

        let playlist = &mut self.playlists[playlist_idx];

        let target_idx = match direction {
            Incrementor::Up if display_idx > 0 => display_idx - 1,
            Incrementor::Down if display_idx < playlist.len() - 1 => display_idx + 1,
            _ => return Ok(()),
        };

        let ps_id1 = playlist.tracklist[display_idx].id;
        let ps_id2 = playlist.tracklist[target_idx].id;

        self.db_worker.swap_position(ps_id1, ps_id2, playlist.id)?;
        playlist.tracklist.swap(display_idx, target_idx);

        self.scroll(match direction {
            Incrementor::Up => Director::Up(1),
            Incrementor::Down => Director::Down(1),
        });

        Ok(())
    }

    fn shift_playlist_position_multi(&mut self, direction: Incrementor) -> Result<()> {
        let mut indices = self
            .get_multi_select_indices()
            .iter()
            .copied()
            .collect::<Vec<_>>();

        indices.sort_unstable();
        let last_selected_idx = indices[indices.len() - 1];

        let Some(playlist_idx) = self.display_state.playlist_pos.selected() else {
            return Ok(());
        };

        let playlist = &mut self.playlists[playlist_idx];
        let playlist_len = playlist.tracklist.len();

        match direction {
            Incrementor::Up if indices[0] > 0 => {
                for idx in indices.iter_mut() {
                    let ps_id1 = playlist.tracklist[*idx].id;
                    let ps_id2 = playlist.tracklist[*idx - 1].id;
                    self.db_worker.swap_position(ps_id1, ps_id2, playlist.id)?;
                    playlist.tracklist.swap(*idx, *idx - 1);
                    *idx -= 1;
                }
            }
            Incrementor::Down if last_selected_idx < (playlist_len - 1) => {
                for idx in indices.iter_mut().rev() {
                    let ps_id1 = playlist.tracklist[*idx].id;
                    let ps_id2 = playlist.tracklist[*idx + 1].id;
                    self.db_worker.swap_position(ps_id1, ps_id2, playlist.id)?;
                    playlist.tracklist.swap(*idx, *idx + 1);
                    *idx += 1;
                }
            }
            // Do nothing but maintain selection in other modes
            _ => return Ok(()),
        }
        self.display_state.multi_select = indices.iter().copied().collect::<IndexSet<_>>();

        Ok(())
    }
}
