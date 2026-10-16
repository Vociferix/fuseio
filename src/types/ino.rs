use std::num::NonZeroU64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, bytemuck::NoUninit)]
#[repr(transparent)]
pub struct Ino(NonZeroU64);

impl Ino {
    pub const ROOT: Self = Self(NonZeroU64::new(1).unwrap());

    pub const fn from_raw(ino: u64) -> Option<Self> {
        if let Some(ino) = NonZeroU64::new(ino) {
            Some(Self(ino))
        } else {
            None
        }
    }

    /// # Safety
    ///
    /// `ino` must not be zero, which the protocol never uses for an inode.
    pub const unsafe fn from_raw_unchecked(ino: u64) -> Self {
        // SAFETY: Non-zero is exactly what this function asks of its caller.
        Self(unsafe { NonZeroU64::new_unchecked(ino) })
    }

    pub const fn as_raw(self) -> u64 {
        self.0.get()
    }
}

impl From<NonZeroU64> for Ino {
    fn from(ino: NonZeroU64) -> Self {
        Self(ino)
    }
}

impl From<Ino> for NonZeroU64 {
    fn from(ino: Ino) -> Self {
        ino.0
    }
}

impl std::fmt::Display for Ino {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::fmt::UpperHex for Ino {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::UpperHex::fmt(&self.0, f)
    }
}

impl std::fmt::LowerHex for Ino {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::LowerHex::fmt(&self.0, f)
    }
}

impl std::fmt::Octal for Ino {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Octal::fmt(&self.0, f)
    }
}

impl std::fmt::Binary for Ino {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Binary::fmt(&self.0, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // `FUSE_ROOT_ID` is one, and every mount starts by looking something up in
    // it.
    #[test]
    fn the_root_is_inode_one() {
        assert_eq!(Ino::ROOT.as_raw(), 1);
        assert_eq!(Ino::from_raw(1), Some(Ino::ROOT));
    }

    // Zero is the protocol's way of saying "no inode", so it can never become
    // one.
    #[test]
    fn zero_is_not_an_inode() {
        assert_eq!(Ino::from_raw(0), None);
    }

    #[test]
    fn every_other_number_round_trips() {
        for raw in [1, 2, 42, u64::MAX] {
            assert_eq!(Ino::from_raw(raw).map(Ino::as_raw), Some(raw));
        }
    }
}
