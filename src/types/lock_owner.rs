use std::num::NonZeroU64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LockOwner(NonZeroU64);

impl From<LockOwner> for u64 {
    fn from(owner: LockOwner) -> Self {
        owner.0.get()
    }
}

impl TryFrom<u64> for LockOwner {
    type Error = std::num::TryFromIntError;

    fn try_from(owner: u64) -> Result<Self, Self::Error> {
        NonZeroU64::try_from(owner).map(Self)
    }
}

impl From<LockOwner> for NonZeroU64 {
    fn from(owner: LockOwner) -> Self {
        owner.0
    }
}

impl From<NonZeroU64> for LockOwner {
    fn from(owner: NonZeroU64) -> Self {
        Self(owner)
    }
}

impl std::fmt::Display for LockOwner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::fmt::UpperHex for LockOwner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::UpperHex::fmt(&self.0, f)
    }
}

impl std::fmt::LowerHex for LockOwner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::LowerHex::fmt(&self.0, f)
    }
}

impl std::fmt::Octal for LockOwner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Octal::fmt(&self.0, f)
    }
}

impl std::fmt::Binary for LockOwner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Binary::fmt(&self.0, f)
    }
}
