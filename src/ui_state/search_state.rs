use super::{Pane, UiState, new_textarea};
use crate::domain::{SimpleSong, SongInfo};
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use ratatui::crossterm::event::KeyEvent;
use std::sync::Arc;
use tui_textarea::TextArea;

const MATCH_THRESHOLD: i64 = 70;

pub(super) struct SearchState {
    pub input: TextArea<'static>,
    matcher: SkimMatcherV2,
}

impl SearchState {
    pub fn new() -> Self {
        SearchState {
            input: new_textarea("Enter search query"),
            matcher: SkimMatcherV2::default(),
        }
    }
}

impl UiState {
    pub(crate) fn filter_songs_by_search(&mut self) {
        let query = self.read_search().to_lowercase();

        let mut scored_songs: Vec<(Arc<SimpleSong>, i64)> = self
            .library
            .get_all_songs()
            .iter()
            .filter_map(|song| {
                // Take the highest score from any field
                let best_score = [
                    self.search
                        .matcher
                        .fuzzy_match(&song.get_title().to_lowercase(), &query),
                    self.search
                        .matcher
                        .fuzzy_match(&song.get_artist().to_lowercase(), &query),
                    self.search
                        .matcher
                        .fuzzy_match(&song.get_album().to_lowercase(), &query),
                ]
                .iter()
                .filter_map(|&score| score)
                .max()
                .unwrap_or(0);

                // let title_score = self
                //     .search
                //     .matcher
                //     .fuzzy_match(&song.get_title().to_lowercase(), &query)
                //     .unwrap_or(0);
                //
                // let artist_score = self
                //     .search
                //     .matcher
                //     .fuzzy_match(&song.get_artist().to_lowercase(), &query)
                //     .unwrap_or(0);
                //
                // let album_score = self
                //     .search
                //     .matcher
                //     .fuzzy_match(&song.get_album().to_lowercase(), &query)
                //     .unwrap_or(0);
                //
                // let best_score = (title_score * 3) + (artist_score * 2) + (album_score * 2);

                if best_score > MATCH_THRESHOLD {
                    Some((Arc::clone(&song), best_score))
                } else {
                    None
                }

                // .filter(|&score| score > MATCH_THRESHOLD)
                // .map(|score| (Arc::clone(&song), score))
            })
            .collect();

        scored_songs.sort_by(|a, b| b.1.cmp(&a.1));

        self.legal_songs = scored_songs.into_iter().map(|i| i.0).collect();
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
        if self.legal_songs.is_empty() {
            self.display_state.table_pos.select(None);
        } else {
            self.display_state.table_pos.select(Some(0));
        }
    }

    pub fn read_search(&self) -> &str {
        &self.search.input.lines()[0]
    }
}
