#[derive(PartialEq)]
pub enum TableSort {
    Title,
    Artist,
    Album,
    Duration,
}

impl ToString for TableSort {
    fn to_string(&self) -> String {
        match self {
            TableSort::Title => "Title".into(),
            TableSort::Artist => "Artist".into(),
            TableSort::Album => "Album".into(),
            TableSort::Duration => "Duration".into(),
        }
    }
}

impl TableSort {
    pub fn next(&self) -> Self {
        match self {
            TableSort::Title => TableSort::Artist,
            TableSort::Artist => TableSort::Album,
            TableSort::Album => TableSort::Duration,
            TableSort::Duration => TableSort::Title,
        }
    }
    pub fn prev(&self) -> Self {
        match self {
            TableSort::Title => TableSort::Duration,
            TableSort::Artist => TableSort::Title,
            TableSort::Album => TableSort::Artist,
            TableSort::Duration => TableSort::Album,
        }
    }
}
