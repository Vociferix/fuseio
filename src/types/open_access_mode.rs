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

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [OpenAccessMode; 3] = [
        OpenAccessMode::ReadOnly,
        OpenAccessMode::WriteOnly,
        OpenAccessMode::ReadWrite,
    ];

    #[test]
    fn every_mode_survives_the_round_trip() {
        for mode in ALL {
            assert_eq!(OpenAccessMode::try_from(OFlag::from(mode)).ok(), Some(mode));
        }
    }

    // The access mode is a small field rather than a set of flags, and
    // `O_RDONLY` is zero, so everything else an open carries has to be masked
    // off before reading it.
    #[test]
    fn the_other_open_flags_do_not_change_the_mode() {
        let noise = OFlag::O_APPEND | OFlag::O_NONBLOCK | OFlag::O_CLOEXEC;

        for mode in ALL {
            assert_eq!(
                OpenAccessMode::try_from(OFlag::from(mode) | noise).ok(),
                Some(mode),
                "{mode:?} was lost among the other flags"
            );
        }
    }

    // What the caller may do follows from the mode, and read-write allows both.
    #[test]
    fn the_permissions_follow_the_mode() {
        assert!(OpenAccessMode::ReadOnly.wants_read());
        assert!(!OpenAccessMode::ReadOnly.wants_write());

        assert!(OpenAccessMode::WriteOnly.wants_write());
        assert!(!OpenAccessMode::WriteOnly.wants_read());

        assert!(OpenAccessMode::ReadWrite.wants_read());
        assert!(OpenAccessMode::ReadWrite.wants_write());
    }
}
