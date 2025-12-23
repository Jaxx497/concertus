pub const GET_WAVEFORM: &str = "
    SELECT waveform FROM waveforms
    WHERE song_id = ?
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
        channels,
        bit_rate,
        sample_rate, 
        format
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13
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
    VALUES (?1, 1)
    ON CONFLICT(song_id) DO UPDATE SET
        count = count + 1 
        WHERE song_id = ?1
";

pub const GET_UI_SNAPSHOT: &str = "
    SELECT key, value 
        FROM session_state 
        WHERE key LIKE 'ui_%'";

pub const SET_SESSION_STATE: &str = "
    INSERT OR REPLACE INTO session_state (key, value)
        VALUES (?, ?)
";

pub const CREATE_NEW_PLAYLIST: &str = "
    INSERT OR IGNORE INTO playlists (name, updated_at) 
        VALUES (?, strftime('%s', 'now'))
";

pub const UPDATE_PLAYLIST: &str = "
    UPDATE playlists
        SET updated_at = strftime('%s', 'now')
        WHERE id = ?
";

pub const DELETE_PLAYLIST: &str = "
    DELETE FROM playlists
        WHERE id = ?
";

pub const GET_PLAYLIST_POSITION_NEXT: &str = "
    SELECT COALESCE(MAX(position), 0)  
    FROM playlist_songs WHERE playlist_id = ?
";

pub const ADD_SONG_TO_PLAYLIST: &str = "
    INSERT INTO playlist_songs (
        song_id, 
        playlist_id, 
        position)
    VALUES (
        ?1, 
        ?2, 
        COALESCE((SELECT MAX(position) + 1
        FROM playlist_songs WHERE playlist_id = ?2), 1)
    )
";

pub const ADD_SONG_TO_PLAYLIST_WITH_POSITION: &str = "
    INSERT INTO playlist_songs (
        song_id, 
        playlist_id, 
        position
    )
    VALUES (
        ?1, 
        ?2, 
        ?3
    )
    
";

pub const PLAYLIST_BUILDER: &str = "
    SELECT 
        ps.id,
        ps.song_id, 
        p.id as playlist_id, 
        p.name 
    FROM playlists p
    LEFT JOIN playlist_songs ps 
        ON p.id = ps.playlist_id
    ORDER BY p.updated_at DESC, COALESCE(ps.position, 0) ASC
";

pub const REMOVE_SONG_FROM_PLAYLIST: &str = "
    DELETE FROM playlist_songs
    WHERE id = ?;
";

pub const GET_PLAYLIST_POS: &str = " 
    SELECT position FROM playlist_songs WHERE id = ?
";

pub const UPDATE_PLAYLIST_POS: &str = "
    UPDATE playlist_songs SET position = ? WHERE id = ?
";

pub const RENAME_PLAYLIST: &str = "
    UPDATE playlists SET name = ? WHERE id = ?
";
