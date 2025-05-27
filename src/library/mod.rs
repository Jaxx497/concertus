mod library;

pub use library::Library;

static LEGAL_EXTENSION: std::sync::LazyLock<std::collections::HashSet<&'static str>> =
    std::sync::LazyLock::new(|| {
        std::collections::HashSet::from(["mp3", "m4a", "flac", "ogg", "wav"])
    });
