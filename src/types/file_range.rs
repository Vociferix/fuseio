#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FileRange {
    Closed(std::ops::RangeInclusive<u64>),
    Open(std::ops::RangeFrom<u64>),
}

impl FileRange {
    pub(crate) const OFFSET_MAX: u64 = 0x7fffffffffffffff;

    pub(crate) fn new(start: u64, end: u64) -> Self {
        if end == Self::OFFSET_MAX {
            Self::Open(start..)
        } else {
            Self::Closed(start..=end)
        }
    }

    pub fn start_offset(&self) -> u64 {
        match self {
            Self::Closed(range) => *range.start(),
            Self::Open(range) => range.start,
        }
    }

    pub fn end_offset(&self) -> Option<u64> {
        if let Self::Closed(range) = self {
            Some(*range.end())
        } else {
            None
        }
    }

    pub fn len(&self) -> Option<u64> {
        if let Self::Closed(range) = self {
            Some(*range.end() - *range.start() + 1)
        } else {
            None
        }
    }
}

impl From<std::ops::Range<u64>> for FileRange {
    fn from(range: std::ops::Range<u64>) -> Self {
        Self::Closed(range.start..=range.end.saturating_sub(1))
    }
}

impl From<std::ops::RangeFrom<u64>> for FileRange {
    fn from(range: std::ops::RangeFrom<u64>) -> Self {
        Self::Open(range)
    }
}

impl From<std::ops::RangeInclusive<u64>> for FileRange {
    fn from(range: std::ops::RangeInclusive<u64>) -> Self {
        Self::Closed(range)
    }
}

impl From<std::ops::RangeTo<u64>> for FileRange {
    fn from(range: std::ops::RangeTo<u64>) -> Self {
        Self::Closed(0..=range.end.saturating_sub(1))
    }
}

impl From<std::ops::RangeToInclusive<u64>> for FileRange {
    fn from(range: std::ops::RangeToInclusive<u64>) -> Self {
        Self::Closed(0..=range.end)
    }
}

impl std::ops::RangeBounds<u64> for FileRange {
    fn start_bound(&self) -> std::ops::Bound<&u64> {
        match self {
            Self::Closed(range) => std::ops::Bound::Included(range.start()),
            Self::Open(range) => std::ops::Bound::Included(&range.start),
        }
    }

    fn end_bound(&self) -> std::ops::Bound<&u64> {
        if let Self::Closed(range) = self {
            std::ops::Bound::Included(range.end())
        } else {
            std::ops::Bound::Unbounded
        }
    }
}
