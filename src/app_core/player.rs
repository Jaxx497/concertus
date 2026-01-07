use anyhow::{anyhow, Result};
use std::sync::Arc;

use crate::{
    app_core::Concertus,
    domain::{SimpleSong, SongDatabase},
    key_handler::SelectionType,
    playback::ValidatedSong,
    player::{ConcertusTrack, PlayerEvent},
    ui_state::{LibraryView, Mode},
};

impl Concertus {
    pub(crate) fn play_song(&mut self, song: &ValidatedSong) -> Result<()> {
        let song = ConcertusTrack::from(song);
        self.player.play(song)
    }

    pub(crate) fn play_selected_song(&mut self) -> Result<()> {
        let song = self.ui.get_selected_song()?;

        if self.ui.get_mode() == &Mode::Queue {
            self.remove_song()?;
        }

        let validated = ValidatedSong::new(&song)?;
        self.play_song(&validated)?;
        self.force_sync()?;

        Ok(())
    }

    pub(crate) fn play_next(&mut self) -> Result<()> {
        match self.advance_to_next() {
            Some(song) => self.play_song(&song)?,
            None => self.player.stop()?,
        }
        self.ui.set_legal_songs();

        Ok(())
    }

    pub(crate) fn play_prev(&mut self) -> Result<()> {
        let (delta, popped) = self
            .ui
            .playback
            .pop_previous()?
            .ok_or_else(|| anyhow!("End of history!"))?;

        self.play_song(&popped)?;
        self.sync_player(&delta);
        self.ui.set_legal_songs();
        Ok(())
    }

    pub fn remove_song(&mut self) -> Result<()> {
        match self.ui.get_mode() {
            Mode::Queue => match self.ui.multi_select_empty() {
                true => self.remove_from_queue()?,
                false => self.remove_from_queue_multi()?,
            },
            Mode::Library(LibraryView::Playlists) => match self.ui.multi_select_empty() {
                true => self.ui.remove_from_playlist()?,
                false => self.ui.remove_from_playlist_multi()?,
            },
            _ => {}
        }
        self.ui.set_legal_songs();
        Ok(())
    }

    pub fn queue_handler(&mut self, selection: Option<Arc<SimpleSong>>) -> Result<()> {
        if !self.ui.multi_select_empty() {
            return self.queue_selection(SelectionType::Multi, false);
        }

        let ss = match selection {
            Some(s) => s,
            None => self.ui.get_selected_song()?,
        };

        match self.player.is_stopped() {
            true => {
                let validated = ValidatedSong::new(&ss)?;
                self.play_song(&validated)?;
            }
            false => self.queue_song(&ss)?,
        }

        self.ui.set_legal_songs();
        Ok(())
    }

    pub(super) fn handle_player_events(&mut self, event: PlayerEvent) -> Result<()> {
        match event {
            PlayerEvent::TrackStarted((this_song, was_gapless)) => {
                let return_id = this_song.id();

                if was_gapless {
                    self.advance_to_next();
                }
                let song = self.library.get_song_by_id(return_id).cloned();
                self.ui.set_now_playing(song);

                if let Some(song) = self.library.get_song_by_id(return_id).cloned() {
                    song.update_play_count()?;
                    self.ui.clear_waveform();
                    self.ui.request_waveform(&song);
                }

                Ok(())
            }
            PlayerEvent::PlaybackStopped => {
                if let Some(next) = self.advance_to_next() {
                    return self.play_song(&next);
                }

                self.ui.playback.set_now_playing(None);
                self.player.stop()?;
                self.ui.clear_waveform();

                self.ui.set_legal_songs();
                Ok(())
            }
            PlayerEvent::Error(e) => {
                self.ui.set_error(anyhow!(e));
                Ok(())
            }
        }
    }
}
