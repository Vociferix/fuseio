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
