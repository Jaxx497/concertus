use crate::{app_core::Concertus, key_handler::Action, ui_state::Mode};
use anyhow::Result;
use crossbeam_channel::Receiver;
use ratatui::crossterm::{self, event::KeyEvent};

impl Concertus {
    #[rustfmt::skip]
    pub fn handle_action(&mut self, action: Action) -> Result<()> {
        match action {
            // Player 
            Action::Play            => self.play_selected_song()?,
            Action::TogglePlayback  => self.player.toggle_playback()?,
            Action::Stop            => self.player.stop()?,
            Action::SeekForward(s)  => self.player.seek_forward(s)?,
            Action::SeekBack(s)     => self.player.seek_back(s)?,
            Action::PlayNext        => self.play_next()?,
            Action::PlayPrev        => self.play_prev()?,

            // UI 
            Action::Scroll(s)       => self.ui.scroll(s),
            Action::GoToAlbum       => self.ui.go_to_album()?,
            Action::ChangeMode(m)   => self.ui.set_mode(m),
            Action::ChangePane(p)   => self.ui.set_pane(p),
            Action::SortColumnsNext => self.ui.next_song_column(),
            Action::SortColumnsPrev => self.ui.prev_song_column(),
            Action::ToggleAlbumSort(next)   => self.ui.toggle_album_sort(next),

            // Search Related
            Action::UpdateSearch(k) => self.ui.process_search(k),
            Action::SendSearch      => self.ui.send_search(),

            //Playlist
            Action::CreatePlaylist  => self.ui.create_playlist_popup(),
            Action::CreatePlaylistConfirm => self.ui.create_playlist()?,

            Action::CreatePlaylistWithSongs => self.ui.create_playlist_with_songs_popup(),
            Action::CreatePlaylistWithSongsConfirm => self.ui.create_playlist_with_songs()?,

            Action::RenamePlaylist  => self.ui.rename_playlist_popup(),
            Action::RenamePlaylistConfirm => self.ui.rename_playlist()?,

            Action::DeletePlaylist  => self.ui.delete_playlist_popup(),
            Action::DeletePlaylistConfirm => self.ui.delete_playlist()?,

            // Queue
            Action::QueueSong       => self.queue_handler(None)?,
            Action::QueueMany{sel_type, shuffle} => self.queue_selection(sel_type, shuffle)?,
            Action::RemoveSong      => self.remove_song()?,
            Action::AddToPlaylist   => self.ui.add_to_playlist_popup(),
            Action::AddToPlaylistConfirm => self.ui.add_to_playlist()?,

            Action::ShuffleElements => self.shuffle_queue(),

            Action::MultiSelect      => self.ui.toggle_multi_selection()?,
            Action::MultiSelectAll   => self.ui.multi_select_all()?,
            Action::ClearMultiSelect => self.ui.clear_multi_select(),

            Action::ShiftPosition(direction) => self.shift_position(direction)?,
            Action::IncrementWFSmoothness(direction) => self.ui.increment_wf_smoothness(direction),
            Action::IncrementSidebarSize(x) => self.ui.adjust_sidebar_size(x),

            Action::SetProgressDisplay(p)   => self.ui.set_progress_display(p),
            Action::SetFullscreen(p)        => self.ui.set_fullscreen(p),
            Action::RevertFullscreen        => self.ui.revert_fullscreen(),

            Action::ThemeRefresh    => self.ui.refresh_current_theme(),
            Action::ThemeManager    => self.ui.open_theme_manager(),
            Action::CycleTheme(dir) => self.ui.cycle_theme(dir),

            // Ops
            Action::PopupInput(key) => self.ui.process_popup_input(&key),
            Action::ClosePopup      => self.ui.close_popup(),
            Action::SoftReset       => self.ui.soft_reset(),
            Action::UpdateLibrary   => self.update_library()?,
            Action::QUIT            => self.ui.set_mode(Mode::QUIT),

            Action::ViewSettings    => self.activate_settings(),
            Action::PopupScrollUp   => self.ui.popup_scroll_up(),
            Action::PopupScrollDown => self.ui.popup_scroll_down(),
            Action::RootAdd         => self.settings_add_root(),
            Action::RootRemove      => self.settings_remove_root(),
            Action::RootConfirm     => self.settings_root_confirm()?,

            _ => (),
        }
        Ok(())
    }
}

pub fn key_loop() -> Receiver<KeyEvent> {
    let (key_tx, key_rx) = crossbeam_channel::bounded(16);

    // 2. SPAWN the input thread (offloading)
    std::thread::spawn(move || loop {
        if let Ok(crossterm::event::Event::Key(key)) = crossterm::event::read() {
            if key.kind == crossterm::event::KeyEventKind::Press {
                let _ = key_tx.try_send(key);
            }
        }
    });

    key_rx
}
