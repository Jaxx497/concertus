use anyhow::{anyhow, Result};
use ratatui::crossterm::{
    cursor::MoveToColumn,
    style::Print,
    terminal::{Clear, ClearType},
    ExecutableCommand,
};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, UNIX_EPOCH},
};
use ui_state::UiState;
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

// ~30fps
pub const REFRESH_RATE: u64 = 33;

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
    let mins = secs / 60;
    secs %= 60;

    match style {
        DurationStyle::Clean => match mins {
            0 => format!("{secs:02}s"),
            _ => format!("{mins}m {secs:02}s"),
        },
        DurationStyle::CleanMillis => match mins {
            0 => format!("{secs:02}s {millis:03}ms"),
            _ => format!("{mins}m {secs:02}sec {millis:02}ms"),
        },
        DurationStyle::Compact => format!("{mins}:{secs:02}"),
        DurationStyle::CompactMillis => format!("{mins}:{secs:02}.{millis:02}"),
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
        return Err(anyhow!("Setting the home directory would read every file in your system. Please provide a more specific path!"));
    }

    if path_str.starts_with("~") || path_str.starts_with("~\\") {
        let home =
            dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory!"))?;
        return Ok(home.join(&path_str[2..]));
    }

    Err(anyhow!("Error reading directory with tilde (~)"))
}
