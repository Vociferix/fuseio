use std::num::NonZeroU64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ino(NonZeroU64);

impl Ino {
    pub const fn from_raw(ino: u64) -> Option<Self> {
        if let Some(ino) = NonZeroU64::new(ino) {
            Some(Self(ino))
        } else {
            None
        }
    }

    pub const unsafe fn from_raw_unchecked(ino: u64) -> Self {
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
