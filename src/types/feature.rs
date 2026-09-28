use super::{ConnCaps, FsCaps, NotifyCaps, PeerCaps, RenameMode};

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

    /// The kernel may notify about watchers of an inode.
    Monitor,

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

    PruneCache,

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
    // The macfuse library lists version 7.45 on the commit where FUSE_MONITOR was added.
    // It's not clear if this is the real minimum protocol version supported by the kext.
    pub(super) const MONITOR: u32 = 45;
    pub(super) const PASSTHROUGH: u32 = 40;
    pub(super) const INC_EPOCH: u32 = 44;
    pub(super) const PRUNE: u32 = 45;
}

/// Answers whether a connection allows a feature.
///
/// `caps` is what is in effect: the features a filesystem enabled once a
/// connection is running, or everything the kernel offered while one is being
/// set up.
pub(crate) fn supports(peer: PeerCaps, feature: Feature, minor_ver: u32, caps: FsCaps) -> bool {
    let version = |since| minor_ver >= since;
    let cap = |cap| caps.contains(cap);
    let conn_cap = |cap| peer.caps.contains(cap);
    let notify_cap = |cap| peer.notify.contains(cap);

    // Only macOS kernels send the requests these answer for, and each is also
    // negotiated, so the capability alone decides.
    const MACOS: bool = cfg!(target_os = "macos");

    match feature {
        Feature::Ioctl => version(since::IOCTL),
        Feature::IoctlOnDirectories => version(since::IOCTL_DIR),
        Feature::Poll => conn_cap(ConnCaps::POLL) && version(since::POLL),
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
                // Reachable only with `EXCHANGE_DATA`, which no other kernel
                // offers, so the capability check above is the whole gate.
                RenameMode::ExchangeData => true,
                // macFUSE carries its rename flags in the request it already
                // had, so it needs no newer protocol version for them.
                _ if MACOS => true,
                _ => conn_cap(ConnCaps::RENAME2) && version(since::RENAME2),
            }
        }

        Feature::Lseek => version(since::LSEEK),
        Feature::CopyFileRange => version(since::COPY_FILE_RANGE),
        Feature::SyncFs => conn_cap(ConnCaps::SYNCFS) && version(since::SYNCFS),
        Feature::TmpFile => version(since::TMPFILE),
        Feature::StatX => version(since::STATX),
        Feature::ExtendedSetxattr => version(since::SETXATTR_EXT) && cap(FsCaps::SETXATTR_EXT),
        Feature::BackupTimes => conn_cap(ConnCaps::BACKUP_TIMES) && cap(FsCaps::MACOS_XTIMES),
        Feature::VolumeName => conn_cap(ConnCaps::VOLUME_NAME) && cap(FsCaps::MACOS_VOL_RENAME),
        Feature::Monitor => conn_cap(ConnCaps::MONITOR) && version(since::MONITOR),

        Feature::InvalidateInode => notify_cap(NotifyCaps::INVAL_INODE) && version(since::INVAL),
        Feature::InvalidateEntry => notify_cap(NotifyCaps::INVAL_ENTRY) && version(since::INVAL),
        Feature::ExpireEntry => notify_cap(NotifyCaps::EXPIRE_ENTRY) && version(since::EXPIRE_ONLY),
        Feature::DeleteEntry => notify_cap(NotifyCaps::DELETE) && version(since::DELETE),
        Feature::StoreCache => notify_cap(NotifyCaps::STORE) && version(since::STORE),
        Feature::RetrieveCache => notify_cap(NotifyCaps::RETRIEVE) && version(since::RETRIEVE),
        Feature::IncrementEpoch => notify_cap(NotifyCaps::INC_EPOCH) && version(since::INC_EPOCH),
        Feature::PollWakeup => notify_cap(NotifyCaps::POLL_WAKEUP) && version(since::NOTIFY_POLL),
        Feature::PruneCache => notify_cap(NotifyCaps::PRUNE) && version(since::PRUNE),

        Feature::Passthrough => {
            conn_cap(ConnCaps::PASSTHROUGH)
                && version(since::PASSTHROUGH)
                && cap(FsCaps::PASSTHROUGH)
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

    /// Asks a peer that gates nothing, leaving the version and the negotiated
    /// capabilities as the only variables.
    fn supported(feature: Feature, minor_ver: u32, caps: FsCaps) -> bool {
        supports(PeerCaps::PERMISSIVE, feature, minor_ver, caps)
    }

    fn supported_by(peer: PeerCaps, feature: Feature, minor_ver: u32, caps: FsCaps) -> bool {
        supports(peer, feature, minor_ver, caps)
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

    // No other kernel offers `EXCHANGE_DATA`, so the negotiated capability is the
    // whole gate and no platform check is needed.
    #[test]
    fn exchanging_data_needs_only_its_capability() {
        let feature = Feature::Rename(RenameMode::ExchangeData);

        assert!(supported(feature, MACOS, FsCaps::EXCHANGE_DATA));
        assert!(!supported(feature, CURRENT, FsCaps::empty()));
    }

    // Only macOS kernels ask for these, which the connection reports rather than
    // the protocol version, so a mock can turn them on anywhere.
    #[test]
    fn the_macos_only_features_follow_the_connection() {
        for (feature, conn, caps) in [
            (
                Feature::BackupTimes,
                ConnCaps::BACKUP_TIMES,
                FsCaps::MACOS_XTIMES,
            ),
            (
                Feature::VolumeName,
                ConnCaps::VOLUME_NAME,
                FsCaps::MACOS_VOL_RENAME,
            ),
        ] {
            assert!(supported(feature, MACOS, caps), "{feature:?}");

            let without = PeerCaps::new(ConnCaps::all().difference(conn), NotifyCaps::all());
            assert!(!supported_by(without, feature, MACOS, caps), "{feature:?}");

            // Still negotiated, even where the connection has them.
            assert!(!supported(feature, MACOS, FsCaps::empty()), "{feature:?}");
        }
    }

    // macFUSE 5 has it and macFUSE 4 doesn't, and nothing negotiates it, so the
    // connection and the protocol version are all that separate them.
    #[test]
    fn monitor_needs_the_connection_and_its_version() {
        assert!(supported(Feature::Monitor, 45, FsCaps::empty()));
        assert!(!supported(Feature::Monitor, MACOS, FsCaps::empty()));

        let without = PeerCaps::new(
            ConnCaps::all().difference(ConnCaps::MONITOR),
            NotifyCaps::all(),
        );
        assert!(!supported_by(
            without,
            Feature::Monitor,
            45,
            FsCaps::empty()
        ));
    }

    // FreeBSD advertises 7.35 but implements neither, so the version alone must
    // not decide these.
    #[test]
    fn a_connection_can_refuse_what_its_version_implies() {
        let peer = PeerCaps::new(ConnCaps::empty(), NotifyCaps::all());

        #[cfg_attr(target_os = "macos", allow(unused_mut))]
        let mut cases = vec![
            (Feature::SyncFs, FsCaps::empty()),
            (Feature::Poll, FsCaps::empty()),
        ];

        // macFUSE needs no `RENAME2` for its flagged renames, so it answers yes
        // whatever the connection says.
        #[cfg(not(target_os = "macos"))]
        cases.push((
            Feature::Rename(RenameMode::NoReplace),
            FsCaps::RENAME_NOREPLACE,
        ));

        for (feature, caps) in cases {
            assert!(supported(feature, CURRENT, caps), "{feature:?}");
            assert!(!supported_by(peer, feature, CURRENT, caps), "{feature:?}");
        }
    }

    // FreeBSD answers only the two invalidations; the rest are `ENOSYS`.
    #[test]
    fn a_connection_can_refuse_a_notification() {
        let peer = PeerCaps::new(
            ConnCaps::all(),
            NotifyCaps::INVAL_INODE.union(NotifyCaps::INVAL_ENTRY),
        );

        for feature in [Feature::InvalidateInode, Feature::InvalidateEntry] {
            assert!(
                supported_by(peer, feature, CURRENT, FsCaps::all()),
                "{feature:?}"
            );
        }

        for feature in [
            Feature::DeleteEntry,
            Feature::StoreCache,
            Feature::RetrieveCache,
            Feature::ExpireEntry,
            Feature::IncrementEpoch,
            Feature::PruneCache,
            Feature::PollWakeup,
        ] {
            assert!(supported(feature, CURRENT, FsCaps::all()), "{feature:?}");
            assert!(
                !supported_by(peer, feature, CURRENT, FsCaps::all()),
                "{feature:?}"
            );
        }
    }

    #[test]
    fn passthrough_needs_the_connection_the_version_and_the_capability() {
        assert!(supported(
            Feature::Passthrough,
            CURRENT,
            FsCaps::PASSTHROUGH
        ));

        let no_passthrough = PeerCaps::new(
            ConnCaps::all().difference(ConnCaps::PASSTHROUGH),
            NotifyCaps::all(),
        );

        assert!(!supported_by(
            no_passthrough,
            Feature::Passthrough,
            CURRENT,
            FsCaps::PASSTHROUGH
        ));
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
