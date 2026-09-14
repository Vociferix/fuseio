use nix::fcntl::OFlag;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OpenAccessMode {
    ReadOnly,
    WriteOnly,
    ReadWrite,
}

impl OpenAccessMode {
    pub const fn is_read_only(self) -> bool {
        matches!(self, Self::ReadOnly)
    }

    pub const fn is_write_only(self) -> bool {
        matches!(self, Self::WriteOnly)
    }

    pub const fn is_read_write(self) -> bool {
        matches!(self, Self::ReadWrite)
    }

    pub const fn wants_read(self) -> bool {
        !self.is_write_only()
    }

    pub const fn wants_write(self) -> bool {
        !self.is_read_only()
    }
}

impl TryFrom<OFlag> for OpenAccessMode {
    type Error = std::io::Error;

    fn try_from(flags: OFlag) -> Result<Self, Self::Error> {
        match flags & OFlag::O_ACCMODE {
            OFlag::O_RDONLY => Ok(Self::ReadOnly),
            OFlag::O_WRONLY => Ok(Self::WriteOnly),
            OFlag::O_RDWR => Ok(Self::ReadWrite),
            _ => Err(std::io::ErrorKind::InvalidInput.into()),
        }
    }
}

impl From<OpenAccessMode> for OFlag {
    fn from(mode: OpenAccessMode) -> Self {
        match mode {
            OpenAccessMode::ReadOnly => OFlag::O_RDONLY,
            OpenAccessMode::WriteOnly => OFlag::O_WRONLY,
            OpenAccessMode::ReadWrite => OFlag::O_RDWR,
        }
    }
}
