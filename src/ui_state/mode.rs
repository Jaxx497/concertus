#[derive(Default, PartialEq, Eq)]
pub enum Mode {
    Album,
    Queue,
    Search,
    QUIT,
    #[default]
    Power,
}

impl PartialEq<Mode> for &Mode {
    fn eq(&self, other: &Mode) -> bool {
        std::mem::discriminant(*self) == std::mem::discriminant(other)
    }
}
