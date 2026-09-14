#[derive(Debug, Clone, Copy)]
pub enum FileTime {
    Specific(std::time::SystemTime),
    Now,
}

impl FileTime {
    pub fn is_now(&self) -> bool {
        matches!(self, Self::Now)
    }

    pub fn is_specific(&self) -> bool {
        matches!(self, Self::Specific(_))
    }

    pub fn time(&self) -> std::time::SystemTime {
        match self {
            Self::Specific(time) => *time,
            Self::Now => std::time::SystemTime::now(),
        }
    }

    pub fn specific_time(&self) -> Option<std::time::SystemTime> {
        match self {
            Self::Specific(time) => Some(*time),
            Self::Now => None,
        }
    }
}

impl From<std::time::SystemTime> for FileTime {
    fn from(time: std::time::SystemTime) -> Self {
        Self::Specific(time)
    }
}

impl From<FileTime> for std::time::SystemTime {
    fn from(time: FileTime) -> std::time::SystemTime {
        time.time()
    }
}
