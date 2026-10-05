//! Whose requests the filesystem answers, which is the half of `allow_root` the
//! kernel does not implement.

use crate::MountOpt;
use crate::proto::request::Opcode;

/// Requests served whoever sent them.
///
/// The first group act on a handle the kernel opened earlier, which is where
/// the caller's right to it was already settled. Several of them also arrive
/// from kernel threads doing writeback or readahead rather than from the
/// process that opened the file, so refusing them would break caching rather
/// than refuse anyone anything.
///
/// The second group take no reply, so a refusal would have nothing to travel
/// in: writing one anyway would put a response on the wire for a request the
/// kernel is not waiting for. The kernel sends all of them with a uid of zero,
/// so they are exempt for that reason too.
const EXEMPT: &[Opcode] = &[
    Opcode::INIT,
    Opcode::READ,
    Opcode::WRITE,
    Opcode::FSYNC,
    Opcode::RELEASE,
    Opcode::READDIR,
    Opcode::FSYNCDIR,
    Opcode::RELEASEDIR,
    Opcode::NOTIFY_REPLY,
    Opcode::READDIRPLUS,
    Opcode::FORGET,
    Opcode::BATCH_FORGET,
    Opcode::INTERRUPT,
    Opcode::MONITOR,
];

/// Whose requests the filesystem serves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Access {
    /// Everyone the kernel lets through, which is the kernel's decision alone.
    Anyone,

    /// Only the owner of the mount and root.
    OwnerOrRoot { owner: u32 },
}

impl Access {
    /// What the mount options ask for.
    ///
    /// [`MountOpt::AllowRoot`] is not a mount option on any platform: the
    /// kernel is asked for `allow_other`, which lets everyone through, and the
    /// narrowing to the owner and root is left to this. So `allow_root` decides
    /// wherever it appears, including alongside `allow_other`, which asks for
    /// no narrowing of its own.
    pub(crate) fn of(options: &[MountOpt]) -> Self {
        if options.contains(&MountOpt::AllowRoot) {
            // The same uid the mounters pass to the kernel as `user_id`.
            Self::OwnerOrRoot {
                owner: nix::unistd::getuid().as_raw(),
            }
        } else {
            Self::Anyone
        }
    }

    /// Whether a request from `uid` for `op` is to be served.
    pub(crate) fn allows(self, uid: u32, op: Opcode) -> bool {
        let Self::OwnerOrRoot { owner } = self else {
            return true;
        };

        // A uid of `FUSE_INVALID_UIDGID` needs no case of its own: an
        // idmapping the kernel could not resolve is neither the owner nor
        // root, and refusing it is the answer either way. Nothing negotiates
        // `FUSE_ALLOW_IDMAP` yet, so it cannot arrive at all for now.
        uid == owner || uid == 0 || EXEMPT.contains(&op)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: u32 = 1000;
    const OTHER: u32 = 1001;

    const NARROW: Access = Access::OwnerOrRoot { owner: OWNER };

    #[test]
    fn without_allow_root_the_kernel_decides_alone() {
        assert_eq!(Access::of(&[]), Access::Anyone);
        assert_eq!(Access::of(&[MountOpt::AllowOther]), Access::Anyone);
    }

    // The narrower option decides, as it does in libfuse, where `allow_root`
    // sets a flag that `allow_other` never clears.
    #[test]
    fn allow_root_decides_even_beside_allow_other() {
        let owner = nix::unistd::getuid().as_raw();

        assert_eq!(
            Access::of(&[MountOpt::AllowRoot]),
            Access::OwnerOrRoot { owner }
        );
        assert_eq!(
            Access::of(&[MountOpt::AllowOther, MountOpt::AllowRoot]),
            Access::OwnerOrRoot { owner }
        );
        assert_eq!(
            Access::of(&[MountOpt::AllowRoot, MountOpt::AllowOther]),
            Access::OwnerOrRoot { owner }
        );
    }

    #[test]
    fn anyone_allows_anyone() {
        assert!(Access::Anyone.allows(OTHER, Opcode::LOOKUP));
    }

    #[test]
    fn the_owner_and_root_are_served() {
        assert!(NARROW.allows(OWNER, Opcode::LOOKUP));
        assert!(NARROW.allows(0, Opcode::LOOKUP));
    }

    #[test]
    fn anyone_else_is_refused() {
        assert!(!NARROW.allows(OTHER, Opcode::LOOKUP));
    }

    // Reading and writing an open file is not where the question is asked, and
    // writeback arrives from a kernel thread rather than from whoever opened
    // the file.
    #[test]
    fn work_on_an_open_handle_is_served_regardless() {
        for op in [
            Opcode::READ,
            Opcode::WRITE,
            Opcode::FSYNC,
            Opcode::RELEASE,
            Opcode::READDIR,
            Opcode::READDIRPLUS,
            Opcode::FSYNCDIR,
            Opcode::RELEASEDIR,
        ] {
            assert!(NARROW.allows(OTHER, op), "{:?} should be served", op.name());
        }
    }

    // A refusal is a reply, so an opcode that takes no reply cannot be refused.
    #[test]
    fn requests_that_take_no_reply_are_never_refused() {
        for op in [
            Opcode::FORGET,
            Opcode::BATCH_FORGET,
            Opcode::INTERRUPT,
            Opcode::MONITOR,
        ] {
            assert!(NARROW.allows(OTHER, op), "{:?} should be served", op.name());
        }
    }

    // The handshake happens before anything is known about who is asking, and
    // refusing it would fail the mount rather than deny a caller.
    #[test]
    fn the_handshake_is_served_regardless() {
        assert!(NARROW.allows(OTHER, Opcode::INIT));
    }

    // An unresolvable idmapping is not the owner, so it is refused like any
    // other stranger.
    #[test]
    fn an_unmappable_uid_is_refused() {
        const FUSE_INVALID_UIDGID: u32 = u32::MAX;

        assert!(!NARROW.allows(FUSE_INVALID_UIDGID, Opcode::LOOKUP));
    }

    // An operation on a path has to be checked: this is the whole point.
    #[test]
    fn path_operations_are_refused() {
        for op in [
            Opcode::LOOKUP,
            Opcode::GETATTR,
            Opcode::SETATTR,
            Opcode::OPEN,
            Opcode::OPENDIR,
            Opcode::UNLINK,
            Opcode::RENAME,
            Opcode::GETXATTR,
            Opcode::SETXATTR,
            Opcode::FLUSH,
            Opcode::STATFS,
        ] {
            assert!(
                !NARROW.allows(OTHER, op),
                "{:?} should be refused",
                op.name()
            );
        }
    }
}
