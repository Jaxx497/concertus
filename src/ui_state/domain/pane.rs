#[derive(Default, PartialEq, Eq, Clone)]
pub enum Pane {
    SideBar,
    Search,
    Popup,
    #[default]
    TrackList,
}

impl PartialEq<Pane> for &Pane {
    fn eq(&self, other: &Pane) -> bool {
        std::mem::discriminant(*self) == std::mem::discriminant(other)
    }
}

impl std::fmt::Display for Pane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Pane::TrackList => write!(f, "tracklist"),
            Pane::SideBar => write!(f, "sidebar"),
            Pane::Search => write!(f, "search"),
            Pane::Popup => write!(f, "temp"),
        }
    }
}

impl Pane {
    pub fn from_str(s: &str) -> Self {
        match s {
            "tracklist" => Pane::TrackList,
            "sidebar" => Pane::SideBar,
            "search" => Pane::Search,
            _ => Pane::TrackList,
        }
    }
}
