/// A span of a file, as locks and cache invalidations describe one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileRange {
    /// A span of `len` bytes, which is empty when `len` is zero.
    Closed { offset: u64, len: u64 },

    /// Everything from `offset` to the end of the file, however the file grows.
    Open { offset: u64 },
}

impl FileRange {
    /// The largest offset a lock can name, which the kernels also use to mean
    /// "to the end of the file".
    pub(crate) const OFFSET_MAX: u64 = i64::MAX as u64;

    /// Builds a range from the inclusive start and end a kernel sends.
    pub(crate) fn new(start: u64, end: u64) -> Self {
        if end >= Self::OFFSET_MAX {
            return Self::Open { offset: start };
        }

        // An end before the start describes nothing.
        let len = end
            .checked_sub(start)
            .and_then(|len| len.checked_add(1))
            .unwrap_or(0);

        Self::Closed { offset: start, len }
    }

    /// Where the range starts.
    pub fn start_offset(&self) -> u64 {
        match self {
            Self::Closed { offset, .. } | Self::Open { offset } => *offset,
        }
    }

    /// The last byte in the range, or [`None`] if it has no end or no bytes.
    pub fn end_offset(&self) -> Option<u64> {
        match self {
            Self::Closed { offset, len } => len.checked_sub(1)?.checked_add(*offset),
            Self::Open { .. } => None,
        }
    }

    /// How many bytes the range covers, or [`None`] if it runs to the end of the
    /// file.
    pub fn len(&self) -> Option<u64> {
        match self {
            Self::Closed { len, .. } => Some(*len),
            Self::Open { .. } => None,
        }
    }

    /// Whether the range covers nothing.
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Closed { len: 0, .. })
    }

    /// The inclusive end to put on the wire, where the largest offset means "to
    /// the end of the file".
    pub(crate) fn wire_end(&self) -> u64 {
        self.end_offset().unwrap_or(Self::OFFSET_MAX)
    }
}

/// Builds a closed range, or an open one when the length can't be represented.
fn closed(offset: u64, len: Option<u64>) -> FileRange {
    match len {
        Some(len) => FileRange::Closed { offset, len },
        None => FileRange::Open { offset },
    }
}

impl From<std::ops::Range<u64>> for FileRange {
    fn from(range: std::ops::Range<u64>) -> Self {
        Self::Closed {
            offset: range.start,
            len: range.end.saturating_sub(range.start),
        }
    }
}

impl From<std::ops::RangeFrom<u64>> for FileRange {
    fn from(range: std::ops::RangeFrom<u64>) -> Self {
        Self::Open {
            offset: range.start,
        }
    }
}

impl From<std::ops::RangeInclusive<u64>> for FileRange {
    fn from(range: std::ops::RangeInclusive<u64>) -> Self {
        let (start, end) = range.into_inner();

        if end < start {
            return Self::Closed {
                offset: start,
                len: 0,
            };
        }

        closed(start, (end - start).checked_add(1))
    }
}

impl From<std::ops::RangeTo<u64>> for FileRange {
    fn from(range: std::ops::RangeTo<u64>) -> Self {
        Self::Closed {
            offset: 0,
            len: range.end,
        }
    }
}

impl From<std::ops::RangeToInclusive<u64>> for FileRange {
    fn from(range: std::ops::RangeToInclusive<u64>) -> Self {
        closed(0, range.end.checked_add(1))
    }
}

impl From<std::ops::RangeFull> for FileRange {
    fn from(_: std::ops::RangeFull) -> Self {
        Self::Open { offset: 0 }
    }
}

impl From<std::range::Range<u64>> for FileRange {
    fn from(range: std::range::Range<u64>) -> Self {
        Self::from(std::ops::Range::from(range))
    }
}

impl From<std::range::RangeFrom<u64>> for FileRange {
    fn from(range: std::range::RangeFrom<u64>) -> Self {
        Self::Open {
            offset: range.start,
        }
    }
}

impl From<std::range::RangeInclusive<u64>> for FileRange {
    fn from(range: std::range::RangeInclusive<u64>) -> Self {
        Self::from(std::ops::RangeInclusive::from(range))
    }
}

impl From<std::range::RangeToInclusive<u64>> for FileRange {
    fn from(range: std::range::RangeToInclusive<u64>) -> Self {
        closed(0, range.last.checked_add(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_half_open_range_keeps_its_length() {
        let range = FileRange::from(4096..8192);

        assert_eq!(range.start_offset(), 4096);
        assert_eq!(range.len(), Some(4096));
        assert_eq!(range.end_offset(), Some(8191));
        assert!(!range.is_empty());
    }

    #[test]
    fn an_inclusive_range_keeps_its_length() {
        let range = FileRange::from(4096..=8191);

        assert_eq!(range.start_offset(), 4096);
        assert_eq!(range.len(), Some(4096));
        assert_eq!(range.end_offset(), Some(8191));
    }

    #[test]
    fn an_empty_range_covers_nothing() {
        for range in [FileRange::from(5..5), FileRange::from(0..0)] {
            assert!(range.is_empty());
            assert_eq!(range.len(), Some(0));
            assert_eq!(range.end_offset(), None);
        }
    }

    #[test]
    fn an_inverted_range_covers_nothing() {
        #[allow(clippy::reversed_empty_ranges)]
        let range = FileRange::from(9..=4);

        assert!(range.is_empty());
        assert_eq!(range.start_offset(), 9);
    }

    #[test]
    fn a_range_with_no_end_has_no_length() {
        for range in [FileRange::from(4096..), FileRange::from(..)] {
            assert_eq!(range.len(), None);
            assert_eq!(range.end_offset(), None);
            assert!(!range.is_empty());
        }
    }

    #[test]
    fn a_range_from_the_start_starts_at_zero() {
        assert_eq!(FileRange::from(..4096).start_offset(), 0);
        assert_eq!(FileRange::from(..4096).len(), Some(4096));
        assert_eq!(FileRange::from(..=4095).len(), Some(4096));
    }

    // Covering every byte is the whole file, which no length can express.
    #[test]
    fn a_range_over_every_byte_has_no_end() {
        for range in [FileRange::from(0..=u64::MAX), FileRange::from(..=u64::MAX)] {
            assert_eq!(range, FileRange::Open { offset: 0 });
            assert_eq!(range.len(), None);
        }
    }

    #[test]
    fn a_range_near_the_largest_offset_doesnt_overflow() {
        let range = FileRange::from(4096..=u64::MAX);

        assert_eq!(range.start_offset(), 4096);
        assert_eq!(range.len(), Some(u64::MAX - 4095));
        assert_eq!(range.end_offset(), Some(u64::MAX));
    }

    #[test]
    fn a_kernels_bounds_become_a_range() {
        assert_eq!(
            FileRange::new(1024, 2047),
            FileRange::Closed {
                offset: 1024,
                len: 1024
            }
        );
        assert_eq!(
            FileRange::new(1024, FileRange::OFFSET_MAX),
            FileRange::Open { offset: 1024 }
        );
    }

    #[test]
    fn a_kernels_inverted_bounds_become_nothing() {
        assert!(FileRange::new(2048, 1024).is_empty());
    }

    #[test]
    fn the_wire_end_is_inclusive() {
        assert_eq!(FileRange::new(1024, 2047).wire_end(), 2047);
        assert_eq!(FileRange::from(1024..).wire_end(), FileRange::OFFSET_MAX);
        // Nothing to lock, so nothing beyond where it starts.
        assert_eq!(
            FileRange::from(1024..1024).wire_end(),
            FileRange::OFFSET_MAX
        );
    }
}
