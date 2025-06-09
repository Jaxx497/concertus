#[derive(Default, PartialEq, Eq)]
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
