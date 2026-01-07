use crate::{
    app_core::Concertus,
    domain::SimpleSong,
    key_handler::{Director, Incrementor, SelectionType},
    playback::{QueueDelta, ValidatedSong},
    player::ConcertusTrack,
    ui_state::{LibraryView, Mode},
};
use anyhow::Result;
use rand::seq::SliceRandom;
use std::sync::Arc;

impl Concertus {
    pub fn advance_to_next(&mut self) -> Option<Arc<ValidatedSong>> {
        let (delta, next) = self.ui.playback.advance();

        if self.ui.get_mode() == Mode::Queue {
            self.ui.set_legal_songs();
        }

        self.sync_player(&delta);
        next
    }

    pub fn queue_song(&mut self, song: &Arc<SimpleSong>) -> Result<()> {
        let delta = self.ui.playback.enqueue(song)?;
        self.sync_player(&delta);
        Ok(())
    }

    pub fn queue_selection(&mut self, sel_type: SelectionType, shuffle: bool) -> Result<()> {
        let mut songs = self.ui.get_songs_by_selection(sel_type)?;
        if songs.is_empty() {
            return Ok(());
        }

        if shuffle {
            songs.shuffle(&mut rand::rng());
        }

        if self.player.is_stopped() {
            let first = songs.remove(0);
            let validated = ValidatedSong::new(&first)?;
            self.play_song(&validated)?;
        }

        let delta = self.ui.playback.enqueue_multi(&songs)?;
        self.sync_player(&delta);
        self.ui.set_legal_songs();
        Ok(())
    }

    pub fn push_queue_front(&mut self, song: &Arc<SimpleSong>) -> Result<()> {
        let delta = self.ui.playback.queue_push_front(song)?;
        self.sync_player(&delta);
        Ok(())
    }

    pub fn shuffle_queue(&mut self) {
        let delta = self.ui.playback.shuffle_queue();
        self.sync_player(&delta);
    }

    pub fn remove_from_queue(&mut self) -> Result<()> {
        let idx = self.ui.get_selected_idx()?;
        let (delta, _popped) = self.ui.playback.remove_from_queue(idx);

        self.sync_player(&delta);
        Ok(())
    }

    pub fn remove_from_queue_multi(&mut self) -> Result<()> {
        let mut indicies = self.ui.get_multi_select_indices().clone();
        indicies.sort_unstable();

        let mut last_delta = QueueDelta::HeadUnchanged;
        for &idx in indicies.iter().rev() {
            let (delta, _) = self.ui.playback.remove_from_queue(idx);
            last_delta = delta;
        }

        self.sync_player(&last_delta);
        self.ui.clear_multi_select();
        Ok(())
    }

    pub fn shift_position(&mut self, dir: Incrementor) -> Result<()> {
        match self.ui.get_mode() {
            Mode::Queue => self.shift_queue_position(dir)?,
            Mode::Library(LibraryView::Playlists) => self.ui.shift_playlist_position(dir)?,
            _ => (),
        }

        self.ui.set_legal_songs();
        Ok(())
    }

    fn shift_queue_position(&mut self, dir: Incrementor) -> Result<()> {
        let delta = match self.ui.multi_select_empty() {
            true => self.shift_qposition_single(dir),
            false => self.shift_qposition_multi(dir),
        };

        if let Some(d) = delta {
            self.sync_player(&d);
        }

        Ok(())
    }

    fn shift_qposition_single(&mut self, dir: Incrementor) -> Option<QueueDelta> {
        let display_idx = self.ui.get_selected_idx().ok()?;

        let target_idx = match dir {
            Incrementor::Up if display_idx > 0 => display_idx - 1,
            Incrementor::Down if display_idx < self.ui.playback.queue_len() - 1 => display_idx + 1,
            _ => return None,
        };

        let delta = self.ui.playback.swap(display_idx, target_idx);
        self.ui.scroll(match dir {
            Incrementor::Up => Director::Up(1),
            Incrementor::Down => Director::Down(1),
        });

        delta
    }

    fn shift_qposition_multi(&mut self, dir: Incrementor) -> Option<QueueDelta> {
        let mut indices = self
            .ui
            .get_multi_select_indices()
            .iter()
            .copied()
            .collect::<Vec<_>>();

        indices.sort_unstable();
        let queue_len = self.ui.playback.queue_len();

        let mut last_delta = None;
        match dir {
            Incrementor::Up if indices[0] > 0 => {
                for idx in indices.iter_mut() {
                    last_delta = self.ui.playback.swap(*idx, *idx - 1);
                    *idx -= 1;
                }
            }
            Incrementor::Down if *indices.last()? < (queue_len - 1) => {
                for idx in indices.iter_mut().rev() {
                    last_delta = self.ui.playback.swap(*idx, *idx + 1);
                    *idx += 1;
                }
            }
            _ => return None,
        }

        self.ui.update_multi_select(indices);
        last_delta
    }

    pub fn force_sync(&self) -> Result<()> {
        let next = self
            .ui
            .playback
            .peek_queue_validated()
            .map(|s| ConcertusTrack::from(s.as_ref()));
        let _ = self.player.set_next(next);
        Ok(())
    }

    /// Ensure that player's up_next value is always synced
    pub fn sync_player(&self, delta: &QueueDelta) {
        if let QueueDelta::HeadChanged { curr, .. } = delta {
            let next = curr.as_ref().map(|s| ConcertusTrack::new(s.id(), s.path()));
            let _ = self.player.set_next(next);
        }
    }
}
