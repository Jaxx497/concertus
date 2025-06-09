#[derive(Default, PartialEq, Eq)]
pub enum Mode {
    Power,
    Queue,
    Search,
    QUIT,
    #[default]
    Album,
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
            Mode::Album => write!(f, "album"),
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
            "album" => Mode::Album,
            "queue" => Mode::Queue,
            "search" => Mode::Search,
            "quit" => Mode::QUIT,
            _ => Mode::Album,
        }
    }
}
