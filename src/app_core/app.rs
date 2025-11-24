use crate::{
    Library,
    app_core::LibraryRefreshProgress,
    domain::{QueueSong, SongDatabase, SongInfo, generate_waveform},
    key_handler::{self},
    overwrite_line,
    player::{PlaybackState, PlayerController},
    tui,
    ui_state::{Mode, PopupType, SettingsMode, UiState},
};
use anyhow::{Result, anyhow, bail};
use ratatui::crossterm::{
    ExecutableCommand,
    event::{
        DisableBracketedPaste, EnableBracketedPaste, Event, KeyEventKind, KeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    },
};
use std::{
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver},
    },
    thread,
    time::Instant,
};

pub struct Concertus {
    _initializer: Instant,
    library: Arc<Library>,
    pub(crate) ui: UiState,
    pub(crate) player: PlayerController,
    waveform_rec: Option<Receiver<Result<Vec<f32>>>>,
    library_refresh_rec: Option<Receiver<LibraryRefreshProgress>>,
}

impl Concertus {
    pub fn new() -> Self {
        let lib = Library::init();
        let lib = Arc::new(lib);
        let lib_clone = Arc::clone(&lib);

        let shared_state = Arc::new(Mutex::new(crate::player::PlayerState::default()));
        let shared_state_clone = Arc::clone(&shared_state);

        Concertus {
            _initializer: Instant::now(),
            library: lib,
            player: PlayerController::new(),
            ui: UiState::new(lib_clone, shared_state_clone),
            waveform_rec: None,
            library_refresh_rec: None,
        }
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let mut terminal = ratatui::init();

        terminal.clear()?;
        std::io::stdout().execute(EnableBracketedPaste)?;
        if cfg!(not(windows)) {
            std::io::stdout().execute(PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES,
            ))?;
        }

        self.preload_lib();
        self.initialize_ui();

        if self.library.roots.is_empty() {
            self.ui
                .show_popup(PopupType::Settings(SettingsMode::AddRoot));
        }

        // MAIN ROUTINE
        loop {
            self.ui.update_player_state(self.player.get_shared_state());

            // Check for user input
            match key_handler::next_event()? {
                Some(Event::Key(key)) if key.kind == KeyEventKind::Press => {
                    // if !self.ui.is_text_input_active() && key_handler::is_likely_paste() {
                    //     continue;
                    // }
                    if let Some(action) = key_handler::handle_key_event(key, &self.ui) {
                        if let Err(e) = self.handle_action(action) {
                            self.ui.set_error(e);
                        }
                    }
                }
                _ => (),
            }

            // If nothing is playing...
            if !self.ui.is_playing() {
                // If there is a song in the queue
                if let Some(song) = self.ui.playback.queue_pop_front() {
                    self.ui.set_playback_state(PlaybackState::Transitioning);
                    if let Err(e) = self.play_song(song) {
                        self.ui.set_error(e);
                    }
                } else {
                    if self.ui.get_mode() == Mode::Fullscreen {
                        self.ui.revert_fullscreen();
                    }
                }
                // Responsive update to queue visual when song ends
                if self.ui.get_mode() == Mode::Queue {
                    self.ui.set_legal_songs();
                }
            }

            let _ = self.await_waveform_completion();
            self.check_library_refresh_progress();

            terminal.draw(|f| tui::render(f, &mut self.ui))?;

            if self.ui.get_mode() == Mode::QUIT {
                self.player.stop()?;
                break;
            }
        }
        std::io::stdout().execute(DisableBracketedPaste)?;
        ratatui::restore();
        overwrite_line("Shutting down... do not close terminal!");
        overwrite_line("Thank you for using concertus!\n\n");

        Ok(())
    }

    fn _debug_startup(&self) {
        let finisher = (Instant::now() - self._initializer).as_secs_f32();
        println!("Finished initializing in {finisher}");
    }

    pub fn preload_lib(&mut self) {
        let mut updated_lib = Library::init();

        updated_lib.build_library().unwrap();

        self.library = Arc::new(updated_lib);
        if let Err(e) = self.ui.sync_library(Arc::clone(&self.library)) {
            self.ui.set_error(e);
        }
    }

    pub fn initialize_ui(&mut self) {
        self.ui.soft_reset();
        self.ui.load_history();
        let _ = self.ui.restore_state();
    }
}

impl Concertus {
    fn play_song(&mut self, song: Arc<QueueSong>) -> Result<()> {
        // Return from function early if selected song is already playing
        if let Some(now_playing) = self.ui.get_now_playing() {
            if now_playing.id == song.get_id() {
                return Ok(());
            }
        }

        if !std::fs::metadata(&song.path).is_ok() {
            bail!("File not found: {}", &song.path);
        }

        self.ui.clear_waveform();
        self.player.play_song(Arc::clone(&song))?;
        self.waveform_handler(&song)?;
        song.update_play_count()?;

        Ok(())
    }

    pub(crate) fn play_selected_song(&mut self) -> Result<()> {
        let song = self.ui.get_selected_song()?;

        if self.ui.get_mode() == &Mode::Queue {
            self.ui.remove_song()?;
        }

        let queue_song = self.ui.make_playable_song(&song)?;

        self.ui.add_to_history(Arc::clone(&song));
        self.play_song(queue_song)
    }

    pub(crate) fn play_next(&mut self) -> Result<()> {
        match self.ui.playback.queue_pop_front() {
            Some(song) => {
                self.ui.add_to_history(Arc::clone(&song.meta));
                self.play_song(song)?;
            }
            None => self.player.stop()?,
        }
        self.ui.set_legal_songs();

        Ok(())
    }

    pub(crate) fn play_prev(&mut self) -> Result<()> {
        match self.ui.get_prev_song() {
            Some(prev) => {
                if let Some(now_playing) = self.ui.get_now_playing() {
                    let queue_song = self.ui.make_playable_song(&now_playing)?;
                    self.ui.playback.queue_push_front(queue_song);
                }
                let queue_song = self.ui.make_playable_song(&prev)?;
                self.play_song(queue_song)?;
            }
            None => self.ui.set_error(anyhow!("End of history!")),
        }

        self.ui.set_legal_songs();
        Ok(())
    }
}

impl Concertus {
    fn waveform_handler(&mut self, song: &QueueSong) -> Result<()> {
        let path_clone = song.path.clone();

        match song.get_waveform() {
            Ok(wf) => {
                self.ui.set_waveform_valid();
                self.ui.set_waveform_visual(wf);
            }
            _ => {
                let (tx, rx) = mpsc::channel();

                thread::spawn(move || {
                    let waveform_res = generate_waveform(&path_clone);
                    let _ = tx.send(waveform_res);
                });
                self.waveform_rec = Some(rx);
            }
        };
        Ok(())
    }

    fn await_waveform_completion(&mut self) -> Result<()> {
        if self.ui.get_waveform_visual().is_empty() && self.ui.get_now_playing().is_some() {
            if let Some(rx) = &self.waveform_rec {
                if let Ok(waveform_result) = rx.try_recv() {
                    let song = self.player.get_now_playing().unwrap();

                    if Some(&song) == self.ui.get_now_playing().as_ref() {
                        match waveform_result {
                            Ok(waveform) => {
                                self.ui.set_waveform_valid();
                                song.set_waveform_db(&waveform)?;
                                self.ui.set_waveform_visual(waveform);
                            }
                            Err(_) => self.ui.set_waveform_invalid(),
                        }
                    }

                    self.waveform_rec = None;
                    return Ok(());
                }
            }
            self.ui.set_waveform_invalid();
            bail!("Invalid waveform");
        }
        Ok(())
    }

    pub(crate) fn update_library(&mut self) -> Result<()> {
        // Don't start another refresh if one is already in progress
        if self.library_refresh_rec.is_some() {
            return Ok(());
        }

        let (tx, rx) = mpsc::channel();
        self.library_refresh_rec = Some(rx);

        // Show initial progress
        self.ui.set_library_refresh_progress(Some(0));

        thread::spawn(move || {
            let _ = tx.send(LibraryRefreshProgress::Scanning { progress: 0 });
            let mut updated_lib = Library::init();

            if updated_lib.roots.is_empty() {
                let _ = tx.send(LibraryRefreshProgress::Complete(updated_lib));
                return;
            }

            let _ = match updated_lib.build_library_with_progress(&tx) {
                Ok(_) => tx.send(LibraryRefreshProgress::Complete(updated_lib)),
                Err(e) => tx.send(LibraryRefreshProgress::Error(e.to_string())),
            };
        });

        Ok(())
    }

    fn check_library_refresh_progress(&mut self) {
        let should_clear = if let Some(rx) = &self.library_refresh_rec {
            match rx.try_recv() {
                Ok(progress) => match progress {
                    LibraryRefreshProgress::Scanning { progress } => {
                        self.ui.set_library_refresh_progress(Some(progress));
                        self.ui
                            .set_library_refresh_detail(Some(format!("Scanning Songs...")));
                        false
                    }
                    LibraryRefreshProgress::Processing {
                        progress,
                        current,
                        total,
                    } => {
                        self.ui.set_library_refresh_progress(Some(progress));
                        self.ui.set_library_refresh_detail(Some(format!(
                            "Processing {}/{}",
                            current, total
                        )));
                        false
                    }
                    LibraryRefreshProgress::UpdatingDatabase { progress } => {
                        self.ui.set_library_refresh_progress(Some(progress));
                        self.ui
                            .set_library_refresh_detail(Some("Updating database...".to_string()));
                        false
                    }
                    LibraryRefreshProgress::Rebuilding { progress } => {
                        self.ui.set_library_refresh_progress(Some(progress));
                        self.ui
                            .set_library_refresh_detail(Some("Rebuilding library...".to_string()));
                        false
                    }
                    LibraryRefreshProgress::Complete(new_library) => {
                        let cached = self.ui.display_state.album_pos.selected();
                        let cached_offset = self.ui.display_state.album_pos.offset();
                        let updated_len = new_library.albums.len();

                        self.library = Arc::new(new_library);
                        if let Err(e) = self.ui.sync_library(Arc::clone(&self.library)) {
                            self.ui.set_error(e);
                        }

                        if updated_len > 0 {
                            self.ui.display_state.album_pos.select(
                                match cached < Some(updated_len) {
                                    true => cached,
                                    false => Some(updated_len / 2),
                                },
                            );
                            *self.ui.display_state.album_pos.offset_mut() = cached_offset;
                        }

                        self.ui.set_legal_songs();
                        self.ui.set_library_refresh_progress(None);
                        self.ui.set_library_refresh_detail(None);
                        true
                    }
                    LibraryRefreshProgress::Error(e) => {
                        self.ui.set_error(anyhow!(e));
                        self.ui.set_library_refresh_progress(None);
                        self.ui.set_library_refresh_detail(None);
                        true
                    }
                },
                Err(mpsc::TryRecvError::Empty) => false,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.ui.set_library_refresh_progress(None);
                    self.ui.set_library_refresh_detail(None);
                    true
                }
            }
        } else {
            false
        };

        if should_clear {
            self.library_refresh_rec = None;
        }
    }
}
