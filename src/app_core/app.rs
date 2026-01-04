use crate::{
    app_core::LibraryRefreshProgress,
    domain::{generate_waveform, QueueSong, SimpleSong, SongDatabase, SongInfo},
    key_handler::{self},
    overwrite_line,
    player2::{ConcertusTrack, PlaybackState, PlayerEvent, PlayerHandle},
    tui,
    ui_state::{Mode, PopupType, SettingsMode, UiState},
    Library,
};
use anyhow::{anyhow, bail, Result};
use ratatui::crossterm::{
    event::{
        DisableBracketedPaste, EnableBracketedPaste, Event, KeyEventKind, KeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    },
    ExecutableCommand,
};
use std::{
    sync::{
        mpsc::{self, Receiver},
        Arc,
    },
    thread,
    time::Instant,
};

pub struct Concertus {
    _initializer: Instant,
    library: Arc<Library>,
    pub(crate) ui: UiState,
    pub(crate) player: PlayerHandle,
    waveform_rec: Option<Receiver<Result<Vec<f32>>>>,
    library_refresh_rec: Option<Receiver<LibraryRefreshProgress>>,
}

impl Concertus {
    pub fn new() -> Self {
        let lib = Library::init();
        let lib = Arc::new(lib);
        let lib_clone = Arc::clone(&lib);

        let player = PlayerHandle::spawn();
        let metrics = player.metrics();

        Concertus {
            _initializer: Instant::now(),
            library: lib,
            player,
            ui: UiState::new(lib_clone, metrics),
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
            for event in self.player.poll_events() {
                if let Err(e) = self.handle_player_events(event) {
                    self.ui.set_error(e);
                }
            }

            match key_handler::next_event()? {
                Some(Event::Key(key)) if key.kind == KeyEventKind::Press => {
                    if let Some(action) = key_handler::handle_key_event(key, &self.ui) {
                        if let Err(e) = self.handle_action(action) {
                            self.ui.set_error(e);
                        }
                    }
                }
                _ => (),
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
        if let Some(now_playing) = self.ui.get_now_playing() {
            if now_playing.id == song.get_id() {
                return Ok(());
            }
        }

        if !std::fs::metadata(&song.path).is_ok() {
            bail!("File not found: {}", &song.path);
        }

        let song = ConcertusTrack::try_from(song.meta.as_ref())?;

        self.player.play(song)?;
        if let Some(up_next) = self.ui.peek_queue() {
            let next_up = ConcertusTrack::try_from(up_next.meta.as_ref())?;
            self.player.set_next(Some(next_up))?;
        };

        Ok(())
    }

    pub fn queue_song(&mut self, song: Option<Arc<SimpleSong>>) -> Result<()> {
        if self.player.is_stopped() {
            let simple_song = match song {
                Some(s) => s,
                None => self.ui.get_selected_song()?,
            };

            let queue_song = QueueSong::from_simple_song(&simple_song)?;
            self.play_song(queue_song)?;
        } else {
            self.ui.queue_song(song)?;
            if self.ui.playback.queue.len() == 1 {
                let x = self.ui.peek_queue().cloned().unwrap();
                let y = ConcertusTrack::try_from(x.meta.as_ref())?;
                let _ = self.player.set_next(Some(y));
            }
        }

        self.ui.set_legal_songs();
        Ok(())
    }

    pub(crate) fn play_selected_song(&mut self) -> Result<()> {
        let song = self.ui.get_selected_song()?;

        if self.ui.get_mode() == &Mode::Queue {
            self.ui.remove_song()?;
        }

        let queue_song = QueueSong::from_simple_song(&song)?;

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
                    let queue_song = QueueSong::from_simple_song(&now_playing)?;
                    self.ui.playback.queue_push_front(queue_song);
                }
                let queue_song = QueueSong::from_simple_song(&prev)?;
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
                    match waveform_result {
                        Ok(waveform) => {
                            self.ui.set_waveform_valid();

                            let song = self.ui.get_now_playing().as_ref().unwrap();

                            song.set_waveform_db(&waveform)?;
                            self.ui.set_waveform_visual(waveform);
                        }
                        Err(_) => self.ui.set_waveform_invalid(),
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

impl Concertus {
    // FIXME: ABSOLUTE FILTH
    fn handle_player_events(&mut self, event: PlayerEvent) -> Result<()> {
        match event {
            PlayerEvent::TrackStarted(return_song) => {
                if let Some(next) = self.ui.peek_queue() {
                    if return_song.get_id() == next.get_id() {
                        if let Some(prev) = self.ui.get_now_playing() {
                            self.ui.add_to_history(Arc::clone(&prev));
                            self.ui.playback.queue_pop_front();

                            if let Some(qs) = self.ui.peek_queue().cloned() {
                                let next = ConcertusTrack::try_from(qs.meta.as_ref()).ok();
                                self.player.set_next(next)?;
                            }
                        }
                    };
                } else {
                    let now_playing = self.library.get_song_by_id(return_song.get_id()).cloned();
                    self.ui.set_now_playing(now_playing)
                }

                if let Some(song) = self.library.get_song_by_id(return_song.get_id()) {
                    self.ui.set_now_playing(Some(Arc::clone(&song)));
                    self.ui.clear_waveform();
                    song.update_play_count()?;
                    let qs = QueueSong::from_simple_song(&song)?;
                    self.waveform_handler(&qs)?;
                }

                Ok(())
            }
            PlayerEvent::PlaybackStopped => {
                // TODO: WE NEED TO HANDLE THE QUEUE

                if let Some(song) = self.ui.get_now_playing() {
                    self.ui.add_to_history(Arc::clone(&song));
                }
                self.ui.set_now_playing(None);

                match self.ui.playback.queue_pop_front() {
                    Some(song) => self.play_song(song)?,
                    None => self.ui.set_playback_state(PlaybackState::Stopped),
                }
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
