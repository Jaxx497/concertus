use crate::{
    domain::{generate_waveform, QueueSong, SongInfo},
    key_handler::{self},
    overwrite_line,
    player::PlayerController,
    tui,
    ui_state::{Mode, PopupType, SettingsMode, UiState},
    Database, Library,
};
use anyhow::{anyhow, Result};
use ratatui::crossterm::event::{Event, KeyEventKind};
use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

pub struct Concertus {
    _initializer: Instant,
    db: Arc<Mutex<Database>>,
    library: Arc<Library>,
    pub(crate) ui: UiState,
    pub(crate) player: PlayerController,
    waveform_rec: Option<mpsc::Receiver<Vec<f32>>>,
    requires_setup: bool,
}

impl Concertus {
    pub fn new() -> Self {
        let db = Database::open().expect("Could not create database!");
        let db = Arc::new(Mutex::new(db));
        let lib = Library::init(Arc::clone(&db));
        let lib = Arc::new(lib);
        let lib_clone = Arc::clone(&lib);

        let shared_state = Arc::new(Mutex::new(crate::player::PlayerState::default()));
        let shared_state_clone = Arc::clone(&shared_state);

        let appstate = Concertus {
            _initializer: Instant::now(),
            db,
            library: lib,
            player: PlayerController::new(),
            ui: UiState::new(lib_clone, shared_state_clone),
            waveform_rec: None,
            requires_setup: true,
        };

        appstate
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let mut terminal = ratatui::init();
        terminal.clear()?;

        self.preload_lib();
        self.initialize_ui();

        match self.requires_setup {
            true => {
                self.ui
                    .show_popup(PopupType::Settings(SettingsMode::AddRoot));
            }
            false => (),
        }

        // MAIN ROUTINE
        loop {
            self.ui.update_player_state(self.player.get_shared_state());

            // Check for user input
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

            // Play next song if song in queue and current song has ended
            if self.ui.is_not_playing() {
                if !self.ui.queue_is_empty() {
                    if let Some(song) = self.ui.playback.queue.pop_front() {
                        if let Err(e) = self.play_song(song) {
                            self.ui.set_error(e);
                        };
                        thread::sleep(Duration::from_millis(75)); // Prevents flickering on waveform widget during song change
                    }
                }
                self.ui.set_legal_songs();
            }

            let _ = self.await_waveform_completion();
            terminal.draw(|f| tui::render(f, &mut self.ui))?;

            if self.ui.get_mode() == Mode::QUIT {
                self.player.stop()?;
                break;
            }
        }
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
        let lib_db = Arc::clone(&self.db);
        let mut updated_lib = Library::init(lib_db);

        if !updated_lib.roots.is_empty() {
            self.requires_setup = false
        };

        // TODO: MAKE THIS OPTIONAL
        // updated_lib.update_db().unwrap();
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
            return Err(anyhow!("File not found: {}", &song.path));
        }

        self.ui.clear_waveform();
        self.waveform_handler(&song)?;
        self.library.update_play_count(&song.meta);
        self.player.play_song(song)?;

        Ok(())
    }

    pub(crate) fn play_selected_song(&mut self) -> Result<()> {
        let song = self.ui.get_selected_song()?;
        let queue_song = self.ui.make_playable_song(&song)?;

        self.ui.add_to_history(Arc::clone(&song));

        self.play_song(queue_song)
    }

    pub(crate) fn play_next(&mut self) -> Result<()> {
        match self.ui.playback.queue.pop_front() {
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
                    self.ui.playback.queue.push_front(queue_song);
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

        let mut db = self.db.lock().unwrap();
        match db.get_waveform(&song.path) {
            Ok(wf) => self.ui.set_waveform(wf),
            _ => {
                let (tx, rx) = mpsc::channel();

                thread::spawn(move || {
                    let waveform = generate_waveform(&path_clone);
                    let _ = tx.send(waveform);
                });
                self.waveform_rec = Some(rx);
            }
        };
        Ok(())
    }

    fn await_waveform_completion(&mut self) -> Result<()> {
        if self.ui.get_waveform().is_empty() && self.ui.get_now_playing().is_some() {
            if let Some(rx) = &self.waveform_rec {
                if let Ok(waveform) = rx.try_recv() {
                    let id = self.player.get_now_playing().unwrap().id;
                    let mut db = self.db.lock().unwrap();

                    db.set_waveform(id, &waveform)?;
                    self.ui.set_waveform(waveform);
                    self.waveform_rec = None;
                    return Ok(());
                }
            }
            return Err(anyhow!("Invalid waveform"));
        }
        Ok(())
    }

    pub(crate) fn update_library(&mut self) -> Result<()> {
        let lib_db = Arc::clone(&self.db);
        let mut updated_lib = Library::init(lib_db);

        let cached = self.ui.display_state.album_pos.selected();
        self.ui.display_state.album_pos.select(None);

        // TODO: Alert user of changes on update
        updated_lib.update_db_by_root()?;
        updated_lib.build_library()?;

        let updated_len = updated_lib.albums.len();

        self.library = Arc::new(updated_lib);
        if let Err(e) = self.ui.sync_library(Arc::clone(&self.library)) {
            self.ui.set_error(e);
        }

        // Do not index a value out of bounds if current selection
        // will be out of bounds after update

        if updated_len > 0 {
            self.ui
                .display_state
                .album_pos
                .select(match cached < Some(updated_len) {
                    true => cached,
                    false => Some(updated_len / 2),
                })
        }

        self.ui.set_legal_songs();

        Ok(())
    }
}
