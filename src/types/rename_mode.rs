/// How a rename request treats its source and target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum RenameMode {
    /// Moves the source to the target, atomically replacing the target if it
    /// exists.
    #[default]
    Replace,

    /// Like [`RenameMode::Replace`], but fails with `EEXIST` if the target
    /// exists.
    ///
    /// Linux `RENAME_NOREPLACE`, macOS `RENAME_EXCL`. Requires
    /// [`FsCaps::RENAME_NOREPLACE`](super::FsCaps::RENAME_NOREPLACE).
    NoReplace,

    /// Atomically exchanges the source and target directory entries, which
    /// must both exist.
    ///
    /// The inodes move with their entries. Linux `RENAME_EXCHANGE`, macOS
    /// `RENAME_SWAP`. Requires
    /// [`FsCaps::RENAME_EXCHANGE`](super::FsCaps::RENAME_EXCHANGE).
    Exchange,

    /// Like [`RenameMode::Replace`], but also creates a whiteout object at the
    /// source.
    ///
    /// Only meaningful for overlay/union filesystems. Linux `RENAME_WHITEOUT`.
    /// Requires [`FsCaps::RENAME_WHITEOUT`](super::FsCaps::RENAME_WHITEOUT).
    Whiteout,

    /// Combines [`RenameMode::Whiteout`] and [`RenameMode::NoReplace`].
    ///
    /// Linux `RENAME_WHITEOUT | RENAME_NOREPLACE`. Requires both
    /// [`FsCaps::RENAME_WHITEOUT`](super::FsCaps::RENAME_WHITEOUT) and
    /// [`FsCaps::RENAME_NOREPLACE`](super::FsCaps::RENAME_NOREPLACE).
    WhiteoutNoReplace,

    /// Atomically exchanges the *contents* of the source and target files
    /// (macOS `exchangedata(2)`).
    ///
    /// Unlike [`RenameMode::Exchange`], the directory entries don't move: each
    /// path keeps its inode number (object identifier), while the file data,
    /// modification time, and open file descriptors move with the data.
    /// Handling this the same way as `Exchange` is incorrect.
    ///
    /// Only sent on macOS. Requires
    /// [`FsCaps::EXCHANGE_DATA`](super::FsCaps::EXCHANGE_DATA).
    ExchangeData,
}

/// Fails for [`RenameMode::ExchangeData`], which has no `renameat2(2)`
/// equivalent.
#[cfg(all(target_os = "linux", target_env = "gnu"))]
impl TryFrom<RenameMode> for nix::fcntl::RenameFlags {
    type Error = std::io::Error;

    fn try_from(mode: RenameMode) -> Result<Self, Self::Error> {
        match mode {
            RenameMode::Replace => Ok(Self::empty()),
            RenameMode::NoReplace => Ok(Self::RENAME_NOREPLACE),
            RenameMode::Exchange => Ok(Self::RENAME_EXCHANGE),
            RenameMode::Whiteout => Ok(Self::RENAME_WHITEOUT),
            RenameMode::WhiteoutNoReplace => Ok(Self::RENAME_WHITEOUT | Self::RENAME_NOREPLACE),
            RenameMode::ExchangeData => Err(std::io::ErrorKind::InvalidInput.into()),
        }
    }
}

/// Fails for flag combinations with no corresponding mode.
#[cfg(all(target_os = "linux", target_env = "gnu"))]
impl TryFrom<nix::fcntl::RenameFlags> for RenameMode {
    type Error = std::io::Error;

    fn try_from(flags: nix::fcntl::RenameFlags) -> Result<Self, Self::Error> {
        use nix::fcntl::RenameFlags;

        if flags.is_empty() {
            Ok(Self::Replace)
        } else if flags == RenameFlags::RENAME_NOREPLACE {
            Ok(Self::NoReplace)
        } else if flags == RenameFlags::RENAME_EXCHANGE {
            Ok(Self::Exchange)
        } else if flags == RenameFlags::RENAME_WHITEOUT {
            Ok(Self::Whiteout)
        } else if flags == RenameFlags::RENAME_WHITEOUT | RenameFlags::RENAME_NOREPLACE {
            Ok(Self::WhiteoutNoReplace)
        } else {
            Err(std::io::ErrorKind::InvalidInput.into())
        }
    }
}
