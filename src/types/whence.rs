// nix only defines `SeekData`/`SeekHole` on these platforms.
#[cfg(any(
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "illumos",
    target_os = "solaris",
    target_os = "hurd",
    target_os = "linux"
))]
use nix::unistd::Whence as NixWhence;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Whence {
    Data,
    Hole,
}

#[cfg(any(
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "illumos",
    target_os = "solaris",
    target_os = "hurd",
    target_os = "linux"
))]
impl TryFrom<NixWhence> for Whence {
    type Error = std::io::Error;

    fn try_from(whence: NixWhence) -> std::result::Result<Self, Self::Error> {
        match whence {
            NixWhence::SeekData => Ok(Self::Data),
            NixWhence::SeekHole => Ok(Self::Hole),
            _ => Err(std::io::ErrorKind::InvalidInput.into()),
        }
    }
}

#[cfg(any(
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "illumos",
    target_os = "solaris",
    target_os = "hurd",
    target_os = "linux"
))]
impl From<Whence> for NixWhence {
    fn from(whence: Whence) -> Self {
        match whence {
            Whence::Data => Self::SeekData,
            Whence::Hole => Self::SeekHole,
        }
    }
}
