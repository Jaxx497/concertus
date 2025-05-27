pub const GET_WAVEFORM: &str = "
    SELECT w.waveform 
    FROM waveforms w
    JOIN songs s on w.song_id = s.id
    WHERE s.path = ?
";

pub const INSERT_WAVEFORM: &str = "
    INSERT or IGNORE INTO waveforms (song_id, waveform)
    VALUES (?1, ?2)
";

pub const GET_ALL_SONGS: &str = "
    SELECT
        s.id,
        s.path,
        s.title,
        s.year,
        s.track_no,
        s.disc_no,
        s.duration,
        s.artist_id,
        s.album_id,
        s.format,
        a.title as album,
        a.artist_id as album_artist
    from songs s
    INNER JOIN albums a ON a.id = s.album_id
    ORDER BY 
        album ASC, 
        disc_no ASC, 
        track_no ASC
";

// KEEP AN EYE ON THIS
// MIGHT REVERT TO INSERT OR IGNORE
pub const INSERT_SONG: &str = "
    INSERT OR REPLACE INTO songs (
        id,
        title, 
        year,
        path, 
        artist_id, 
        album_id, 
        track_no, 
        disc_no, 
        duration, 
        sample_rate, 
        format
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11
)";

pub const INSERT_ARTIST: &str = "
    INSERT OR IGNORE INTO artists (
    name
) VALUES (?1)
";

pub const INSERT_ALBUM: &str = "
    INSERT OR IGNORE INTO albums (
    title,
    artist_id
) VALUES (?1, ?2)
";

// pub const GET_SONGS: &str = "
//     SELECT
//         s.id,
//         s.title as title,
//         ar.name as artist,
//         al.title as album,
//         art_album.name as album_artist,
//         s.track_no,
//         s.disc_no,
//         s.duration
//     FROM songs s
//     LEFT JOIN artists ar ON ar.id = s.artist_id
//     LEFT JOIN albums al ON al.id = s.album_id;
//     LEFT JOIN artists art_album ON art_album.id = al.artist_id
// ";

pub const GET_PATH: &str = "
    SELECT path FROM songs
    WHERE id = ?
";

pub const GET_ARTIST_MAP: &str = "
    SELECT id, name FROM artists
";

pub const GET_ALBUM_MAP: &str = "
    SELECT id, title, artist_id FROM albums
";

pub const ALBUM_BUILDER: &str = "
    SELECT 
        id, artist_id 
    FROM albums
    ORDER BY title
";

pub const GET_ROOTS: &str = "
    SELECT path FROM roots
";

pub const SET_ROOT: &str = "
    INSERT OR IGNORE INTO roots (path) VALUES (?)
";

pub const DELETE_ROOT: &str = "
    DELETE FROM roots WHERE path = ?
";

pub const GET_HASHES: &str = "
    SELECT id FROM songs
";

pub const DELETE_SONGS: &str = "
    DELETE FROM songs WHERE id = ?
";

pub const LOAD_HISTORY: &str = "
    SELECT song_id FROM history
    ORDER BY timestamp DESC
    LIMIT 50
";

pub const INSERT_INTO_HISTORY: &str = "
    INSERT INTO history (song_id, timestamp) VALUES (?, ?)";

pub const DELETE_FROM_HISTORY: &str = "
    DELETE FROM history WHERE id NOT IN 
        (SELECT id FROM history ORDER BY timestamp DESC LIMIT 50)
";

pub const UPDATE_PLAY_COUNT: &str = "
    INSERT INTO plays 
        (song_id, count)
    VALUES (?1, ?2)
    ON CONFLICT(song_id) DO UPDATE SET
        count = count + ?2 
        WHERE song_id = ?1
";
