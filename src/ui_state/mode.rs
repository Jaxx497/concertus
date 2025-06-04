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
