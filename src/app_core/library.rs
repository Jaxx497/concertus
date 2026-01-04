use crate::{
    app_core::{Concertus, LibraryRefreshProgress},
    Library,
};
use anyhow::{anyhow, Result};
use std::{sync::Arc, thread};

impl Concertus {
    pub(crate) fn update_library(&mut self) -> Result<()> {
        if self.library_refresh_rec.is_some() {
            return Ok(());
        }

        let (tx, rx) = crossbeam_channel::bounded(1);
        self.library_refresh_rec = Some(rx);

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

    pub(super) fn handle_library_progress(&mut self, progress: LibraryRefreshProgress) {
        match progress {
            LibraryRefreshProgress::Scanning { progress } => {
                self.ui.set_library_refresh_progress(Some(progress));
                self.ui
                    .set_library_refresh_detail(Some(format!("Scanning Songs...")));
            }
            LibraryRefreshProgress::Processing {
                progress,
                current,
                total,
            } => {
                self.ui.set_library_refresh_progress(Some(progress));
                self.ui
                    .set_library_refresh_detail(Some(format!("Processing {}/{}", current, total)));
            }
            LibraryRefreshProgress::UpdatingDatabase { progress } => {
                self.ui.set_library_refresh_progress(Some(progress));
                self.ui
                    .set_library_refresh_detail(Some("Updating database...".to_string()));
            }
            LibraryRefreshProgress::Rebuilding { progress } => {
                self.ui.set_library_refresh_progress(Some(progress));
                self.ui
                    .set_library_refresh_detail(Some("Rebuilding library...".to_string()));
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
                    self.ui
                        .display_state
                        .album_pos
                        .select(match cached < Some(updated_len) {
                            true => cached,
                            false => Some(updated_len / 2),
                        });
                    *self.ui.display_state.album_pos.offset_mut() = cached_offset;
                }

                self.ui.set_legal_songs();
                self.ui.set_library_refresh_progress(None);
                self.ui.set_library_refresh_detail(None);
                self.library_refresh_rec = None;
            }
            LibraryRefreshProgress::Error(e) => {
                self.ui.set_error(anyhow!(e));
                self.ui.set_library_refresh_progress(None);
                self.ui.set_library_refresh_detail(None);
                self.library_refresh_rec = None;
            }
        }
    }
}
