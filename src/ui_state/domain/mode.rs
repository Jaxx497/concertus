#[derive(Default, PartialEq, Eq, Clone, Copy)]
pub enum LibraryView {
    #[default]
    Albums,
    Playlists,
}

#[derive(PartialEq, Eq, Clone)]
pub enum Mode {
    Power,
    Library(LibraryView),
    Fullscreen,
    Queue,
    Search,
    QUIT,
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Library(LibraryView::default())
    }
}

impl PartialEq<Mode> for &Mode {
    fn eq(&self, other: &Mode) -> bool {
        std::mem::discriminant(*self) == std::mem::discriminant(other)
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Power => write!(f, "power"),
            Mode::Library(LibraryView::Albums) => write!(f, "library_album"),
            Mode::Library(LibraryView::Playlists) => write!(f, "library_playlist"),
            Mode::Fullscreen => write!(f, "fullscreen"),
            Mode::Queue => write!(f, "queue"),
            Mode::Search => write!(f, "search"),
            Mode::QUIT => write!(f, "quit"),
        }
    }
}

impl Mode {
    pub fn from_str(s: &str) -> Self {
        match s {
            "power" => Mode::Power,
            "library_album" => Mode::Library(LibraryView::Albums),
            "library_playlist" => Mode::Library(LibraryView::Playlists),
            "queue" => Mode::Queue,
            "search" => Mode::Search,
            "quit" => Mode::QUIT,
            _ => Mode::Library(LibraryView::Albums),
        }
    }
}
