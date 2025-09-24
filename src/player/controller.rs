use super::{PlaybackState, Player, PlayerCommand, PlayerState};
use crate::domain::{QueueSong, SimpleSong};
use anyhow::Result;
use std::{
    sync::{
        Arc, Mutex,
        mpsc::{self, Sender},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

pub struct PlayerController {
    sender: Sender<PlayerCommand>,
    shared_state: Arc<Mutex<PlayerState>>,
    _thread_handle: JoinHandle<()>,
}

impl PlayerController {
    pub fn new() -> Self {
        let (sender, reciever) = mpsc::channel();
        let shared_state = Arc::new(Mutex::new(PlayerState::default()));
        let shared_state_clone = Arc::clone(&shared_state);

        let thread_handle = thread::spawn(move || {
            let mut player = Player::new(shared_state_clone);

            loop {
                if let Ok(message) = reciever.try_recv() {
                    match message {
                        PlayerCommand::Play(song) => {
                            if let Err(e) = player.play_song(&song) {
                                let mut state = player.shared_state.lock().unwrap();

                                state.player_error = Some(e)
                            }
                        }
                        PlayerCommand::TogglePlayback => player.toggle_playback(),
                        PlayerCommand::SeekForward(secs) => {
                            player
                                .seek_forward(secs)
                                .unwrap_or_else(|e| eprintln!("Error: {e}"));
                        }
                        PlayerCommand::SeekBack(secs) => player.seek_back(secs),
                        PlayerCommand::Stop => player.stop(),
                    };
                }

                match player.sink_is_empty() {
                    true => player.stop(),
                    false => player.update_elapsed(),
                }
                // Lessen cpu intensity, but avoid stutters between songs
                thread::sleep(Duration::from_millis(16))
            }
        });

        PlayerController {
            sender,
            shared_state,
            _thread_handle: thread_handle,
        }
    }

    pub fn play_song(&self, song: Arc<QueueSong>) -> Result<()> {
        self.sender.send(PlayerCommand::Play(song))?;
        Ok(())
    }

    pub fn toggle_playback(&self) -> Result<()> {
        self.sender.send(PlayerCommand::TogglePlayback)?;
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        self.sender.send(PlayerCommand::Stop)?;
        Ok(())
    }

    pub fn seek_forward(&self, s: usize) -> Result<()> {
        self.sender.send(PlayerCommand::SeekForward(s))?;
        Ok(())
    }

    pub fn seek_back(&self, s: usize) -> Result<()> {
        self.sender.send(PlayerCommand::SeekBack(s))?;
        Ok(())
    }

    pub fn get_now_playing(&self) -> Option<Arc<SimpleSong>> {
        let state = self.shared_state.lock().unwrap();
        state.now_playing.clone()
    }

    /// Get the elapsed time of a song
    pub fn get_elapsed(&self) -> Duration {
        let state = self.shared_state.lock().unwrap();
        state.elapsed
    }

    pub fn is_paused(&self) -> bool {
        let state = self.shared_state.lock().unwrap();
        state.state == PlaybackState::Paused
    }

    pub fn sink_is_empty(&self) -> bool {
        let state = self.shared_state.lock().unwrap();
        state.now_playing.is_none() || state.state == PlaybackState::Stopped
    }

    pub fn get_shared_state(&self) -> Arc<Mutex<PlayerState>> {
        Arc::clone(&self.shared_state)
    }
}
