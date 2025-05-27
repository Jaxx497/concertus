#[derive(Default, PartialEq, Eq, Clone, Copy)]
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
