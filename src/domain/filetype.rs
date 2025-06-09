use rusqlite::{
    types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, Value, ValueRef},
    Result as RusqliteResult, ToSql,
};
use std::fmt::Display;

#[allow(clippy::upper_case_acronyms)]
#[derive(Default, PartialEq, Copy, Clone)]
pub enum FileType {
    MP3 = 1,
    M4A = 2,
    OGG = 3,
    WAV = 4,
    FLAC = 5,
    #[default]
    ERR = 0,
}

impl From<&str> for FileType {
    fn from(str: &str) -> Self {
        match str {
            "mp3" => Self::MP3,
            "m4a" => Self::M4A,
            "ogg" => Self::OGG,
            "flac" => Self::FLAC,
            "wav" => Self::WAV,
            _ => Self::ERR,
        }
    }
}

impl FromSql for FileType {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value {
            ValueRef::Integer(i) => Ok(FileType::from_i64(i)),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

impl ToSql for FileType {
    fn to_sql(&self) -> RusqliteResult<rusqlite::types::ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Integer(self.to_i64())))
    }
}

impl Display for FileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            FileType::MP3 => write!(f, "ᵐᵖ³"),
            FileType::M4A => write!(f, "ᵐ⁴ᵃ"),
            FileType::OGG => write!(f, "ᵒᵍᵍ"),
            FileType::WAV => write!(f, "ʷᵃᵛ"),
            FileType::FLAC => write!(f, "ᶠˡᵃᶜ"),
            FileType::ERR => write!(f, "ERR"),
        }
    }
}

impl FileType {
    pub fn from_i64(value: i64) -> Self {
        match value {
            1 => Self::MP3,
            2 => Self::M4A,
            3 => Self::OGG,
            4 => Self::WAV,
            5 => Self::FLAC,
            _ => Self::ERR,
        }
    }

    pub fn to_i64(&self) -> i64 {
        *self as i64
    }
}
