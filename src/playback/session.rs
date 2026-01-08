use crate::{
    Database, SongMap,
    library::SimpleSong,
    playback::{HISTORY_CAPACITY, QueueDelta, ValidatedSong},
};
use anyhow::Result;
use rand::seq::SliceRandom;
use std::{
    collections::{HashSet, VecDeque},
    sync::Arc,
};

pub struct PlaybackSession {
    queue: VecDeque<Arc<ValidatedSong>>,
    history: VecDeque<Arc<SimpleSong>>,
    queue_ids: HashSet<u64>,

    now_playing: Option<Arc<SimpleSong>>,
}

impl PlaybackSession {
    pub fn init() -> Self {
        PlaybackSession {
            queue: VecDeque::new(),
            history: VecDeque::with_capacity(HISTORY_CAPACITY),
            queue_ids: HashSet::new(),
            now_playing: None,
        }
    }

    pub fn peek_queue(&self) -> Option<&Arc<SimpleSong>> {
        self.queue.front().map(|s| &s.meta)
    }

    pub fn peek_queue_validated(&self) -> Option<&Arc<ValidatedSong>> {
        self.queue.front()
    }

    pub fn get_now_playing(&self) -> Option<&Arc<SimpleSong>> {
        self.now_playing.as_ref()
    }

    pub fn set_now_playing(&mut self, song: Option<Arc<SimpleSong>>) {
        self.now_playing = song
    }

    pub fn get_queue(&mut self) -> Vec<Arc<SimpleSong>> {
        self.queue
            .make_contiguous()
            .iter()
            .map(|s| Arc::clone(&s.meta))
            .collect()
    }

    pub fn export_history(&mut self) -> Vec<u64> {
        self.history
            .make_contiguous()
            .iter()
            .map(|s| s.id)
            .collect()
    }

    // =====================
    //    QUEUE METHODS
    // =====================
    pub fn enqueue(&mut self, song: &Arc<SimpleSong>) -> Result<QueueDelta> {
        let validated = ValidatedSong::new(song)?;
        let prev = self.get_head();

        self.queue_ids.insert(validated.id());
        self.queue.push_back(validated);

        Ok(self.head_delta(prev))
    }

    pub fn enqueue_multi(&mut self, songs: &[Arc<SimpleSong>]) -> Result<QueueDelta> {
        let prev = self.get_head();

        for song in songs {
            if let Ok(validated) = ValidatedSong::new(song) {
                self.queue_ids.insert(validated.id());
                self.queue.push_back(validated);
            }
        }

        Ok(self.head_delta(prev))
    }

    /// Push song to front of queue
    pub fn queue_push_front(&mut self, song: &Arc<SimpleSong>) -> Result<QueueDelta> {
        let validated = ValidatedSong::new(song)?;
        let prev = self.get_head();

        self.queue_ids.insert(validated.id());
        self.queue.push_front(Arc::clone(&validated));

        Ok(QueueDelta::HeadChanged {
            prev,
            curr: Some(validated),
        })
    }

    /// Take now_playing, put to history.
    pub fn advance(&mut self) -> (QueueDelta, Option<Arc<ValidatedSong>>) {
        let prev = self.get_head();

        if let Some(current) = self.now_playing.take() {
            self.push_history(&current);
        }

        let next = self.queue.pop_front().map(|song| {
            self.remove_id_if_final(song.id());
            song
        });

        (self.head_delta(prev), next)
    }

    pub fn remove_from_queue(&mut self, idx: usize) -> (QueueDelta, Option<Arc<ValidatedSong>>) {
        let prev = self.get_head();
        let dropped = self.queue.remove(idx).map(|s| {
            self.remove_id_if_final(s.id());
            s
        });

        (self.head_delta(prev), dropped)
    }

    pub fn swap(&mut self, a: usize, b: usize) -> Option<QueueDelta> {
        if a.max(b) >= self.queue.len() || a == b {
            return None;
        }

        let prev = self.get_head();
        self.queue.swap(a, b);
        Some(self.head_delta(prev))
    }

    pub fn shuffle_queue(&mut self) -> QueueDelta {
        let prev = self.get_head();
        self.queue.make_contiguous().shuffle(&mut rand::rng());
        self.head_delta(prev)
    }

    pub fn is_queued(&self, id: u64) -> bool {
        self.queue_ids.contains(&id)
    }

    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    pub fn queue_is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    // ======================
    //    HISTORY METHODS
    // ======================
    //

    pub(crate) fn load_history(&mut self, song_map: &SongMap) -> Result<()> {
        let mut db = Database::open()?;
        self.history = db.import_history(song_map)?;
        Ok(())
    }

    pub fn push_history(&mut self, song: &Arc<SimpleSong>) {
        self.history.push_front(Arc::clone(song));
        if self.history.len() > HISTORY_CAPACITY {
            self.history.pop_back();
        }
    }

    pub fn pop_previous(&mut self) -> Result<Option<(QueueDelta, Arc<ValidatedSong>)>> {
        // Handle nothing in history
        let last_played = match self.history.pop_front() {
            Some(song) => song,
            None => return Ok(None),
        };

        let prev_head = self.get_head();

        // If something is playing, place it back in the queue
        if let Some(current) = self.now_playing.take() {
            let validated = ValidatedSong::new(&current)?;
            self.queue_ids.insert(validated.id());
            self.queue.push_front(validated);
        }

        // Validate what was popped, set as now playing
        let validated_popped = ValidatedSong::new(&last_played)?;
        self.now_playing = Some(last_played);

        let delta = self.head_delta(prev_head);

        Ok(Some((delta, validated_popped)))
    }

    // ======================
    //    INTERNAL METHODS
    // ======================

    fn get_head(&self) -> Option<Arc<ValidatedSong>> {
        self.queue.front().cloned()
    }

    fn head_delta(&self, prev: Option<Arc<ValidatedSong>>) -> QueueDelta {
        let curr = self.queue.front().cloned();
        let curr_id = curr.as_ref().map(|s| s.id());
        let prev_id = prev.as_ref().map(|s| s.id());

        match curr_id == prev_id {
            true => QueueDelta::HeadUnchanged,
            false => QueueDelta::HeadChanged { prev, curr },
        }
    }

    fn remove_id_if_final(&mut self, id: u64) {
        if !self.queue.iter().any(|s| s.id() == id) {
            self.queue_ids.remove(&id);
        }
    }
}
