pub const CREATE_TABLES: &str = r"
    CREATE TABLE IF NOT EXISTS roots(
        id INTEGER PRIMARY KEY,
        path TEXT UNIQUE NOT NULL
    );

    CREATE TABLE IF NOT EXISTS songs(
        id BLOB PRIMARY KEY,
        title TEXT NOT NULL,
        year INTEGER,
        path TEXT UNIQUE NOT NULL,
        artist_id INTEGER,
        album_id INTEGER,
        track_no INTEGER,
        disc_no INTEGER,
        duration REAL,
        channels INTEGER,
        bit_rate INTEGER,
        sample_rate INTEGER,
        format INTEGER,
        FOREIGN KEY(artist_id) REFERENCES artists(id),
        FOREIGN KEY(album_id) REFERENCES albums(id)
    );

    CREATE TABLE IF NOT EXISTS artists(
        id INTEGER PRIMARY KEY,
        name TEXT UNIQUE NOT NULL
    );

    CREATE TABLE IF NOT EXISTS albums(
        id INTEGER PRIMARY KEY,
        title TEXT NOT NULL,
        artist_id INTEGER,
        FOREIGN KEY(artist_id) REFERENCES artists(id),
        UNIQUE (title, artist_id)
    );

    CREATE TABLE IF NOT EXISTS waveforms(
        song_id BLOB PRIMARY KEY,
        waveform BLOB,
        FOREIGN KEY(song_id) REFERENCES songs(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS history(
        id INTEGER PRIMARY KEY,
        song_id BLOB NOT NULL,
        timestamp INTEGER NOT NULL,
        FOREIGN KEY(song_id) REFERENCES songs(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS plays(
        song_id BLOB PRIMARY KEY,
        count INTEGER,
        FOREIGN KEY(song_id) REFERENCES songs(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS session_state(
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS playlists(
        id INTEGER PRIMARY KEY,
        name TEXT UNIQUE NOT NULL,
        updated_at INTEGER NOT NULL
    );

    CREATE TABLE IF NOT EXISTS playlist_songs(
        id INTEGER PRIMARY KEY,
        song_id BLOB NOT NULL,
        playlist_id INTEGER NOT NULL,
        position INTEGER NOT NULL,
        FOREIGN KEY (song_id) REFERENCES songs(id) ON DELETE CASCADE,
        FOREIGN KEY (playlist_id) REFERENCES playlists(id) ON DELETE CASCADE,
        UNIQUE(playlist_id, position)
    );
";
