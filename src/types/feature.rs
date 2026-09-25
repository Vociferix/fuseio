use super::{FsCaps, RenameMode};

/// Something a filesystem may want to know is available before relying on it.
///
/// What a connection allows depends on the protocol version the kernel speaks,
/// on which features were negotiated, and in a few cases on the platform. Asking
/// through [`Context::supports`](crate::fs::types::Context::supports) folds all
/// three together, so a filesystem doesn't have to know which decides a given
/// feature.
///
/// Features that are purely negotiated, such as writeback caching, are answered
/// by [`Context::caps`](crate::fs::types::Context::caps) instead: the filesystem
/// chose those itself, and a mount fails outright if it asks for one the kernel
/// doesn't have.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Feature {
    /// The kernel may send `ioctl` requests.
    Ioctl,

    /// The kernel may send `ioctl` requests for directories.
    IoctlOnDirectories,

    /// The kernel may send `poll` requests.
    Poll,

    /// The kernel may forget several inodes in one request.
    BatchForget,

    /// The kernel may ask for space to be allocated or punched out.
    Fallocate,

    /// The kernel may read a directory and its entries' attributes together.
    ReadDirPlus,

    /// The kernel may ask for a rename of this kind.
    Rename(RenameMode),

    /// The kernel may ask where the next data or hole in a file is.
    Lseek,

    /// The kernel may ask for a range to be copied between two files.
    CopyFileRange,

    /// The kernel may ask for the whole filesystem to be flushed.
    SyncFs,

    /// The kernel may ask for an unnamed file.
    TmpFile,

    /// The kernel may ask for attributes through `statx`, with a field mask.
    StatX,

    /// Extended attribute requests carry the flags that say whether to clear
    /// setgid.
    ExtendedSetxattr,

    /// The kernel may ask for a file's backup and creation times.
    ///
    /// Only macOS has these.
    BackupTimes,

    /// The kernel may ask for the volume to be renamed.
    ///
    /// Only macOS asks.
    VolumeName,

    /// Cached attributes and data can be invalidated.
    InvalidateInode,

    /// A cached directory entry can be invalidated.
    InvalidateEntry,

    /// A cached directory entry can be marked stale without dropping it.
    ExpireEntry,

    /// A directory entry can be reported as deleted, waking anything watching
    /// the parent.
    DeleteEntry,

    /// Data can be put straight into the kernel's cache.
    StoreCache,

    /// Data can be read back out of the kernel's cache.
    RetrieveCache,

    /// Every cached directory entry can be invalidated at once.
    IncrementEpoch,

    /// A pending `poll` can be woken.
    PollWakeup,

    /// Reads and writes can be handed to a backing file.
    ///
    /// Only Linux has this, and only when built for it. Even where the
    /// connection allows it, opening a backing file still needs privileges, a
    /// regular file, and a filesystem that isn't already deeply stacked, so this
    /// saying `true` doesn't promise the next attempt succeeds.
    Passthrough,

    /// The kernel speaks at least this minor version of the protocol.
    ///
    /// For a filesystem that knows the protocol better than this list does.
    ProtocolAtLeast(u32),
}

/// The protocol version each feature arrived in.
mod since {
    pub(super) const NOTIFY_POLL: u32 = 11;
    pub(super) const IOCTL: u32 = 11;
    pub(super) const POLL: u32 = 11;
    pub(super) const INVAL: u32 = 12;
    pub(super) const STORE: u32 = 15;
    pub(super) const RETRIEVE: u32 = 15;
    pub(super) const BATCH_FORGET: u32 = 16;
    pub(super) const IOCTL_DIR: u32 = 18;
    pub(super) const DELETE: u32 = 18;
    pub(super) const FALLOCATE: u32 = 19;
    pub(super) const READDIRPLUS: u32 = 21;
    pub(super) const RENAME2: u32 = 23;
    pub(super) const LSEEK: u32 = 24;
    pub(super) const COPY_FILE_RANGE: u32 = 28;
    pub(super) const SETXATTR_EXT: u32 = 33;
    pub(super) const SYNCFS: u32 = 34;
    pub(super) const TMPFILE: u32 = 37;
    pub(super) const EXPIRE_ONLY: u32 = 38;
    pub(super) const STATX: u32 = 39;
    pub(super) const PASSTHROUGH: u32 = 40;
    pub(super) const INC_EPOCH: u32 = 44;
}

/// Answers whether a connection allows a feature.
///
/// `caps` is what is in effect: the features a filesystem enabled once a
/// connection is running, or everything the kernel offered while one is being
/// set up.
pub(crate) fn supports(feature: Feature, minor_ver: u32, caps: FsCaps) -> bool {
    let version = |since| minor_ver >= since;
    let cap = |cap| caps.contains(cap);
    let macos = cfg!(target_os = "macos");

    match feature {
        Feature::Ioctl => version(since::IOCTL),
        Feature::IoctlOnDirectories => version(since::IOCTL_DIR),
        Feature::Poll => version(since::POLL),
        Feature::BatchForget => version(since::BATCH_FORGET),
        Feature::Fallocate => version(since::FALLOCATE) && cap(FsCaps::FALLOCATE),
        Feature::ReadDirPlus => version(since::READDIRPLUS) && cap(FsCaps::DO_READDIRPLUS),

        // macOS carries its rename flags in the request it already had, so it
        // needs no newer protocol version for them.
        Feature::Rename(RenameMode::Replace) => true,
        Feature::Rename(mode) => {
            let caps_needed = match mode {
                RenameMode::Replace => FsCaps::empty(),
                RenameMode::NoReplace => FsCaps::RENAME_NOREPLACE,
                RenameMode::Exchange => FsCaps::RENAME_EXCHANGE,
                RenameMode::Whiteout => FsCaps::RENAME_WHITEOUT,
                RenameMode::WhiteoutNoReplace => {
                    FsCaps::RENAME_WHITEOUT.union(FsCaps::RENAME_NOREPLACE)
                }
                RenameMode::ExchangeData => FsCaps::EXCHANGE_DATA,
            };

            if !caps.contains(caps_needed) {
                return false;
            }

            match mode {
                RenameMode::ExchangeData => macos,
                _ if macos => true,
                _ => version(since::RENAME2),
            }
        }

        Feature::Lseek => version(since::LSEEK),
        Feature::CopyFileRange => version(since::COPY_FILE_RANGE),
        Feature::SyncFs => version(since::SYNCFS),
        Feature::TmpFile => version(since::TMPFILE),
        Feature::StatX => version(since::STATX),
        Feature::ExtendedSetxattr => version(since::SETXATTR_EXT) && cap(FsCaps::SETXATTR_EXT),
        Feature::BackupTimes => macos && cap(FsCaps::MACOS_XTIMES),
        Feature::VolumeName => macos && cap(FsCaps::MACOS_VOL_RENAME),

        Feature::InvalidateInode | Feature::InvalidateEntry => version(since::INVAL),
        Feature::ExpireEntry => version(since::EXPIRE_ONLY),
        Feature::DeleteEntry => version(since::DELETE),
        Feature::StoreCache => version(since::STORE),
        Feature::RetrieveCache => version(since::RETRIEVE),
        Feature::IncrementEpoch => version(since::INC_EPOCH),
        Feature::PollWakeup => version(since::NOTIFY_POLL),

        Feature::Passthrough => {
            cfg!(target_os = "linux") && version(since::PASSTHROUGH) && cap(FsCaps::PASSTHROUGH)
        }

        Feature::ProtocolAtLeast(minor) => version(minor),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CURRENT: u32 = crate::handshake::MINOR_VER;

    /// The version every macOS kernel speaks.
    const MACOS: u32 = 19;

    fn supported(feature: Feature, minor_ver: u32, caps: FsCaps) -> bool {
        supports(feature, minor_ver, caps)
    }

    #[test]
    fn a_current_kernel_allows_the_version_gated_features() {
        for feature in [
            Feature::Ioctl,
            Feature::IoctlOnDirectories,
            Feature::Poll,
            Feature::BatchForget,
            Feature::Lseek,
            Feature::CopyFileRange,
            Feature::SyncFs,
            Feature::TmpFile,
            Feature::StatX,
            Feature::InvalidateInode,
            Feature::InvalidateEntry,
            Feature::ExpireEntry,
            Feature::DeleteEntry,
            Feature::StoreCache,
            Feature::RetrieveCache,
            Feature::IncrementEpoch,
            Feature::PollWakeup,
        ] {
            assert!(supported(feature, CURRENT, FsCaps::empty()), "{feature:?}");
        }
    }

    #[test]
    fn an_older_kernel_allows_less() {
        assert!(!supported(Feature::StatX, 38, FsCaps::empty()));
        assert!(!supported(Feature::SyncFs, 33, FsCaps::empty()));
        assert!(!supported(Feature::IncrementEpoch, 43, FsCaps::empty()));
        assert!(!supported(Feature::Lseek, 23, FsCaps::empty()));
        assert!(!supported(Feature::IoctlOnDirectories, 17, FsCaps::empty()));
    }

    // The macOS kernels speak 7.19, so most of the later work is out of reach.
    #[test]
    fn a_macos_kernel_allows_the_older_features() {
        assert!(supported(Feature::Ioctl, MACOS, FsCaps::empty()));
        assert!(supported(Feature::InvalidateInode, MACOS, FsCaps::empty()));
        assert!(supported(Feature::DeleteEntry, MACOS, FsCaps::empty()));
        assert!(!supported(
            Feature::ReadDirPlus,
            MACOS,
            FsCaps::DO_READDIRPLUS
        ));
        assert!(!supported(Feature::StatX, MACOS, FsCaps::empty()));
        assert!(!supported(Feature::ExpireEntry, MACOS, FsCaps::empty()));
    }

    #[test]
    fn a_feature_needs_both_its_version_and_its_capability() {
        assert!(!supported(Feature::Fallocate, CURRENT, FsCaps::empty()));
        assert!(supported(Feature::Fallocate, CURRENT, FsCaps::FALLOCATE));
        assert!(!supported(Feature::Fallocate, 18, FsCaps::FALLOCATE));

        assert!(!supported(
            Feature::ExtendedSetxattr,
            CURRENT,
            FsCaps::empty()
        ));
        assert!(supported(
            Feature::ExtendedSetxattr,
            CURRENT,
            FsCaps::SETXATTR_EXT
        ));
    }

    #[test]
    fn a_plain_rename_always_works() {
        assert!(supported(
            Feature::Rename(RenameMode::Replace),
            11,
            FsCaps::empty()
        ));
    }

    #[test]
    fn a_flagged_rename_needs_its_capability() {
        for (mode, caps) in [
            (RenameMode::NoReplace, FsCaps::RENAME_NOREPLACE),
            (RenameMode::Exchange, FsCaps::RENAME_EXCHANGE),
            (RenameMode::Whiteout, FsCaps::RENAME_WHITEOUT),
        ] {
            assert!(
                !supported(Feature::Rename(mode), CURRENT, FsCaps::empty()),
                "{mode:?}"
            );
            assert!(supported(Feature::Rename(mode), CURRENT, caps), "{mode:?}");
        }

        assert!(!supported(
            Feature::Rename(RenameMode::WhiteoutNoReplace),
            CURRENT,
            FsCaps::RENAME_WHITEOUT
        ));
        assert!(supported(
            Feature::Rename(RenameMode::WhiteoutNoReplace),
            CURRENT,
            FsCaps::RENAME_WHITEOUT | FsCaps::RENAME_NOREPLACE
        ));
    }

    #[test]
    fn exchanging_data_is_a_macos_rename() {
        assert_eq!(
            supported(
                Feature::Rename(RenameMode::ExchangeData),
                CURRENT,
                FsCaps::EXCHANGE_DATA
            ),
            cfg!(target_os = "macos")
        );
    }

    #[test]
    fn the_macos_only_features_follow_the_platform() {
        for (feature, caps) in [
            (Feature::BackupTimes, FsCaps::MACOS_XTIMES),
            (Feature::VolumeName, FsCaps::MACOS_VOL_RENAME),
        ] {
            assert_eq!(
                supported(feature, MACOS, caps),
                cfg!(target_os = "macos"),
                "{feature:?}"
            );
            assert!(!supported(feature, MACOS, FsCaps::empty()), "{feature:?}");
        }
    }

    #[test]
    fn passthrough_needs_the_platform_the_version_and_the_capability() {
        assert_eq!(
            supported(Feature::Passthrough, CURRENT, FsCaps::PASSTHROUGH),
            cfg!(target_os = "linux")
        );
        assert!(!supported(Feature::Passthrough, CURRENT, FsCaps::empty()));
        assert!(!supported(Feature::Passthrough, 39, FsCaps::PASSTHROUGH));
    }

    #[test]
    fn a_version_can_be_asked_for_directly() {
        assert!(supported(
            Feature::ProtocolAtLeast(31),
            CURRENT,
            FsCaps::empty()
        ));
        assert!(!supported(
            Feature::ProtocolAtLeast(CURRENT + 1),
            CURRENT,
            FsCaps::empty()
        ));
    }
}
