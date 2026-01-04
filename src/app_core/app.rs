use crate::{
    app_core::{key_loop, Concertus},
    overwrite_line,
    player::PlayerHandle,
    tui,
    ui_state::{Mode, PopupType, SettingsMode, UiState},
    Library,
};
use ratatui::crossterm::{
    event::{
        DisableBracketedPaste, EnableBracketedPaste, KeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    },
    ExecutableCommand,
};
use std::sync::Arc;

impl Concertus {
    pub fn new() -> Self {
        let lib = Arc::new(Library::init());
        let lib_clone = Arc::clone(&lib);

        let player = PlayerHandle::spawn();
        let metrics = player.metrics();

        Concertus {
            library: lib,
            player,
            ui: UiState::new(lib_clone, metrics),
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

        let key_rx = key_loop();

        // ================
        //   MAIN ROUTINE
        // ================
        loop {
            self.select_shortcut(&key_rx);

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
        let _ = self.ui.playback.load_history(self.library.get_songs_map());
        let _ = self.ui.restore_state();
    }
}
