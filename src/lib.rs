use anyhow::{Result, anyhow, bail};
use indexmap::IndexMap;
use nohash_hasher::BuildNoHashHasher;
use ratatui::crossterm::{
    ExecutableCommand,
    cursor::MoveToColumn,
    style::Print,
    terminal::{Clear, ClearType},
};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, UNIX_EPOCH},
};
use ui_state::UiState;
use unicode_normalization::UnicodeNormalization;
use xxhash_rust::xxh3::xxh3_64;

pub mod app_core;
pub mod database;
pub mod domain;
pub mod key_handler;
pub mod library;
pub mod player;
pub mod tui;
pub mod ui_state;

pub use database::Database;
pub use library::Library;
pub use player::Player;

use crate::domain::SimpleSong;

pub type SongMap = IndexMap<u64, Arc<SimpleSong>, BuildNoHashHasher<u64>>;

// ~120fps
pub const REFRESH_RATE: Duration = Duration::from_millis(8);
pub const CONFIG_DIRECTORY: &'static str = "concertus";
pub const THEME_DIRECTORY: &'static str = "themes";
pub const DATABASE_FILENAME: &'static str = "concertus.db";

/// Create a hash based on...
///  - date of last modification (millis)
///  - file size (bytes)
///  - path as str as bytes
pub fn calculate_signature<P: AsRef<Path>>(path: P) -> anyhow::Result<u64> {
    let metadata = fs::metadata(&path)?;

    let last_mod = metadata.modified()?.duration_since(UNIX_EPOCH)?.as_millis() as i64;
    let size = metadata.len();

    let mut data = Vec::with_capacity(path.as_ref().as_os_str().len() + 16);

    data.extend_from_slice(path.as_ref().as_os_str().as_encoded_bytes());
    data.extend_from_slice(&last_mod.to_le_bytes());
    data.extend_from_slice(&size.to_le_bytes());

    Ok(xxh3_64(&data))
}

pub enum DurationStyle {
    Clean,
    CleanMillis,
    Compact,
    CompactMillis,
}

pub fn get_readable_duration(duration: Duration, style: DurationStyle) -> String {
    let mut secs = duration.as_secs();
    let millis = duration.subsec_millis() % 100;
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    secs %= 60;

    match style {
        DurationStyle::Clean => match hours {
            0 => match mins {
                0 => format!("{secs:02}s"),
                _ => format!("{mins}m {secs:02}s"),
            },
            _ => format!("{hours}h {mins}m {secs:02}s"),
        },
        DurationStyle::CleanMillis => match hours {
            0 => match mins {
                0 => format!("{secs:02}s {millis:03}ms"),
                _ => format!("{mins}m {secs:02}sec {millis:02}ms"),
            },
            _ => format!("{hours}h {mins}m {secs:02}sec {millis:02}ms"),
        },
        DurationStyle::Compact => match hours {
            0 => format!("{mins}:{secs:02}"),
            _ => format!("{hours}:{mins:02}:{secs:02}"),
        },
        DurationStyle::CompactMillis => match hours {
            0 => format!("{mins}:{secs:02}.{millis:02}"),
            _ => format!("{hours}:{mins:02}:{secs:02}.{millis:02}"),
        },
    }
}

fn truncate_at_last_space(s: &str, limit: usize) -> String {
    if s.chars().count() <= limit {
        return s.to_string();
    }

    let byte_limit = s
        .char_indices()
        .map(|(i, _)| i)
        .nth(limit)
        .unwrap_or(s.len());

    match s[..byte_limit].rfind(' ') {
        Some(last_space) => {
            let mut truncated = s[..last_space].to_string();
            truncated.push('…');
            truncated
        }
        None => {
            let char_boundary = s[..byte_limit]
                .char_indices()
                .map(|(i, _)| i)
                .last()
                .unwrap_or(0);

            let mut truncated = s[..char_boundary].to_string();
            truncated.push('…');
            truncated
        }
    }
}

pub fn normalize_metadata_str(s: &str) -> String {
    s.nfc()
        .filter(|c| match c {
            '\0' | '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{00AD}' | '\u{2028}' | '\u{2029}' => {
                false
            }
            '\n' | '\t' => true,
            c if c.is_control() => false,
            _ => true,
        })
        .collect::<String>()
        .trim() // Only once!
        .to_string()
}

pub fn strip_win_prefix(path: &str) -> String {
    let path_str = path.to_string();
    path_str
        .strip_prefix(r"\\?\")
        .unwrap_or(&path_str)
        .to_string()
}

pub fn overwrite_line(message: &str) {
    let mut stdout = std::io::stdout();
    stdout
        .execute(MoveToColumn(0))
        .unwrap()
        .execute(Clear(ClearType::CurrentLine))
        .unwrap()
        .execute(Print(message))
        .unwrap();
    stdout.flush().unwrap();
}

pub fn expand_tilde<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
    let path = path.as_ref();
    let path_str = path.to_string_lossy();

    if !path_str.starts_with('~') {
        return Ok(path.to_path_buf());
    }

    if path_str == "~" {
        bail!(
            "Setting the home directory would read every file in your system. Please provide a more specific path!"
        );
    }

    if path_str.starts_with("~") || path_str.starts_with("~\\") {
        let home =
            dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory!"))?;
        return Ok(home.join(&path_str[2..]));
    }

    Err(anyhow!("Error reading directory with tilde (~)"))
}

pub fn get_random_playlist_idea() -> &'static str {
    use rand::seq::IndexedRandom;

    match PLAYLIST_IDEAS.choose(&mut rand::rng()) {
        Some(s) => s,
        None => "",
    }
}

const PLAYLIST_IDEAS: [&str; 46] = [
    "A Lantern in the Dark",
    "A Map Without Places",
    "After the Rain Ends",
    "Background Music for Poor Decisions",
    "Beats Me, Literally",
    "Certified Hood Classics (But It’s Just Me Singing)",
    "Chordially Yours",
    "Clouds Made of Static",
    "Coffee Shop Apocalypse",
    "Ctrl Alt Repeat",
    "Dancing on the Edge of Stillness",
    "Drifting Into Tomorrow",
    "Echoes Between Stars",
    "Existential Karaoke",
    "Fragments of a Dream",
    "Frequencies Between Worlds",
    "Ghosts of Tomorrow’s Sunlight",
    "Horizons That Never End",
    "I Liked It Before It Was Cool",
    "In Treble Since Birth",
    "Key Changes and Life Changes",
    "Liminal Grooves",
    "Low Effort High Vibes",
    "Major Minor Issues",
    "Melancholy But Make It Funky",
    "Microwave Symphony",
    "Midnight Conversations",
    "Music to Stare Dramatically Out the Window To",
    "Neon Memories in Sepia",
    "Note to Self",
    "Notes From Another Dimension",
    "Off-Brand Emotions™",
    "Rhythm & Clues",
    "Sharp Notes Only",
    "Silence Speaks Louder",
    "Songs Stuck Between Pages",
    "Songs That Owe Me Rent",
    "Soundtrack for Imaginary Films",
    "Tempo Tantrums",
    "Temporary Eternity",
    "The Shape of Sound to Come",
    "The Weight of Quiet",
    "Untranslatable Feelings",
    "Vinyl Countdown",
    "Waiting for the Beat to Drop (Forever)",
    "When the World Pauses",
];
