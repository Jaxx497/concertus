#[derive(PartialEq, Eq)]
pub enum ProgressDisplay {
    Waveform,
    ProgressBar,
    Oscilloscope,
}

impl ProgressDisplay {
    pub fn from_str(s: &str) -> Self {
        match s {
            "progress_bar" => Self::ProgressBar,
            "oscilloscope" => Self::Oscilloscope,
            _ => Self::Waveform,
        }
    }
}

impl std::fmt::Display for ProgressDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProgressDisplay::Waveform => write!(f, "waveform"),
            ProgressDisplay::ProgressBar => write!(f, "progress_bar"),
            ProgressDisplay::Oscilloscope => write!(f, "oscilloscope"),
        }
    }
}
