use super::SFlag;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InodeKind {
    Dir,
    File,
    Symlink,
    BlockDev,
    CharDev,
    Fifo,
    Socket,
}

impl From<InodeKind> for SFlag {
    fn from(kind: InodeKind) -> Self {
        match kind {
            InodeKind::Dir => SFlag::S_IFDIR,
            InodeKind::File => SFlag::S_IFREG,
            InodeKind::Symlink => SFlag::S_IFLNK,
            InodeKind::BlockDev => SFlag::S_IFBLK,
            InodeKind::CharDev => SFlag::S_IFCHR,
            InodeKind::Fifo => SFlag::S_IFIFO,
            InodeKind::Socket => SFlag::S_IFSOCK,
        }
    }
}

impl TryFrom<SFlag> for InodeKind {
    type Error = std::io::Error;

    fn try_from(flag: SFlag) -> Result<Self, Self::Error> {
        match flag & SFlag::S_IFMT {
            SFlag::S_IFDIR => Ok(Self::Dir),
            SFlag::S_IFREG => Ok(Self::File),
            SFlag::S_IFLNK => Ok(Self::Symlink),
            SFlag::S_IFBLK => Ok(Self::BlockDev),
            SFlag::S_IFCHR => Ok(Self::CharDev),
            SFlag::S_IFIFO => Ok(Self::Fifo),
            SFlag::S_IFSOCK => Ok(Self::Socket),
            _ => Err(std::io::ErrorKind::InvalidInput.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [InodeKind; 7] = [
        InodeKind::Dir,
        InodeKind::File,
        InodeKind::Symlink,
        InodeKind::BlockDev,
        InodeKind::CharDev,
        InodeKind::Fifo,
        InodeKind::Socket,
    ];

    #[test]
    fn every_kind_survives_the_round_trip() {
        for kind in ALL {
            assert_eq!(InodeKind::try_from(SFlag::from(kind)).ok(), Some(kind));
        }
    }

    // No two kinds may share a format bit pattern, or one would be mistaken for
    // another on the way back.
    #[test]
    fn the_kinds_are_distinct_on_the_wire() {
        for (i, a) in ALL.into_iter().enumerate() {
            for b in ALL.into_iter().skip(i + 1) {
                assert_ne!(SFlag::from(a), SFlag::from(b), "{a:?} and {b:?}");
            }
        }
    }

    // A mode carries permissions alongside the format, and only the format
    // says what the inode is.
    #[test]
    fn the_permission_bits_are_not_part_of_the_kind() {
        let mode = SFlag::S_IFREG | SFlag::from_bits_retain(0o7777);

        assert_eq!(InodeKind::try_from(mode).ok(), Some(InodeKind::File));
    }

    // A mode with no format bits describes nothing this crate can name.
    #[test]
    fn a_mode_with_no_format_is_refused() {
        let err = InodeKind::try_from(SFlag::from_bits_retain(0o644)).unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    }
}
