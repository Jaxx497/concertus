#[derive(PartialEq, Eq, Clone, Copy)]
pub enum AlbumSort {
    Artist,
    Title,
    Year,
}

impl ToString for AlbumSort {
    fn to_string(&self) -> String {
        match self {
            AlbumSort::Artist => "Artist".into(),
            AlbumSort::Title => "Title".into(),
            AlbumSort::Year => "Year".into(),
        }
    }
}

impl PartialEq<AlbumSort> for &AlbumSort {
    fn eq(&self, other: &AlbumSort) -> bool {
        std::mem::discriminant(*self) == std::mem::discriminant(other)
    }
}

impl AlbumSort {
    pub fn next(&self) -> AlbumSort {
        match self {
            AlbumSort::Artist => AlbumSort::Title,
            AlbumSort::Title => AlbumSort::Year,
            AlbumSort::Year => AlbumSort::Artist,
        }
    }

    pub fn prev(&self) -> AlbumSort {
        match self {
            AlbumSort::Artist => AlbumSort::Year,
            AlbumSort::Title => AlbumSort::Artist,
            AlbumSort::Year => AlbumSort::Title,
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Artist" => AlbumSort::Artist,
            "Title" => AlbumSort::Title,
            "Year" => AlbumSort::Year,
            _ => AlbumSort::Artist,
        }
    }
}
