use super::{Pane, UiState, new_textarea};
use crate::library::{SimpleSong, SongInfo};
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use ratatui::crossterm::event::KeyEvent;
use std::{collections::HashMap, sync::Arc};
use tui_textarea::TextArea;

const MATCH_THRESHOLD: i64 = 80;
const MATCH_LIMIT: usize = 250;

#[derive(Copy, Clone)]
pub enum MatchField {
    Title,
    Artist,
    Album,
}

pub struct SearchState {
    pub input: TextArea<'static>,
    matcher: SkimMatcherV2,
    pub(super) match_fields: HashMap<u64, MatchField>,
}

impl SearchState {
    pub fn new() -> Self {
        SearchState {
            input: new_textarea("Enter search query"),
            matcher: SkimMatcherV2::default(),
            match_fields: HashMap::new(),
        }
    }
}

impl UiState {
    // Algorithm looks at the title, artist, and album fields
    // and scores each attribute while applying a heavier
    // weight to the title field and returns the highest score.
    // Assuming the score is higher than the threshold, the
    // result is valid. Results are ordered by score.
    pub(crate) fn filter_songs_by_search(&mut self) {
        let query = self.read_search().to_lowercase();

        let mut scored_songs: Vec<(Arc<SimpleSong>, i64)> = self
            .library
            .get_all_songs()
            .iter()
            .filter_map(|song| {
                let title_score = self
                    .search
                    .matcher
                    .fuzzy_match(&song.get_title().to_lowercase(), &query)
                    .unwrap_or(0)
                    * 2;

                let artist_score = (self
                    .search
                    .matcher
                    .fuzzy_match(&song.get_artist().to_lowercase(), &query)
                    .unwrap_or(0) as f32
                    * 1.5) as i64;

                let album_score = (self
                    .search
                    .matcher
                    .fuzzy_match(&song.get_album().to_lowercase(), &query)
                    .unwrap_or(0) as f32
                    * 1.75) as i64;

                let weighted_score = [title_score, artist_score, album_score];
                let best_score = weighted_score.iter().max().copied().unwrap_or(0);

                if best_score > MATCH_THRESHOLD {
                    let match_field = if best_score == title_score {
                        MatchField::Title
                    } else if best_score == artist_score {
                        MatchField::Artist
                    } else {
                        MatchField::Album
                    };
                    self.search.match_fields.insert(song.get_id(), match_field);
                }

                (best_score > MATCH_THRESHOLD).then(|| (Arc::clone(&song), best_score))
            })
            .collect();

        scored_songs.sort_by(|a, b| b.1.cmp(&a.1));
        self.legal_songs = scored_songs
            .into_iter()
            .take(MATCH_LIMIT)
            .map(|i| i.0)
            .collect();
    }

    pub fn get_search_widget(&mut self) -> &mut TextArea<'static> {
        &mut self.search.input
    }

    pub fn get_search_len(&self) -> usize {
        self.search.input.lines()[0].len()
    }

    pub fn send_search(&mut self) {
        match !self.legal_songs.is_empty() {
            true => self.set_pane(Pane::TrackList),
            false => self.soft_reset(),
        }
    }

    pub fn process_search(&mut self, k: KeyEvent) {
        self.search.input.input(k);
        self.set_legal_songs();
        match self.legal_songs.is_empty() {
            true => self.display_state.table_pos.select(None),
            false => self.display_state.table_pos.select(Some(0)),
        }
    }

    pub fn read_search(&self) -> &str {
        &self.search.input.lines()[0]
    }

    pub fn get_match_fields(&self, song_id: u64) -> Option<MatchField> {
        self.search.match_fields.get(&song_id).copied()
    }
}
