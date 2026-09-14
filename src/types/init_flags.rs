//! INIT request/reply flags, normalized to one layout across platforms.
//!
//! Linux and the BSDs share Linux's bit assignments, with bits 32..=63 carried
//! in `flags2` when `INIT_EXT` is set. macOS kexts speak protocol 7.19 and reuse
//! bits 23..=31 of `flags` for Darwin flags, colliding with Linux flags added
//! after 7.19. Normalized, Darwin flags sit at their wire bit + 32 (55..=63), so
//! every flag has a unique position on every platform.
//!
//! The public `KernelCaps` and `FsCaps` share these positions.

use super::FsCaps;

const LINUX_MASK: u64 = (1 << 55) - 1;
const DARWIN_WIRE_MASK: u32 = !((1 << 23) - 1);
const DARWIN_SHIFT: u32 = 32;

macro_rules! init_flags {
    ($($(#[$attr:meta])* struct $name:ident;)+) => {$(
        bitflags::bitflags! {
            $(#[$attr])*
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
            pub(crate) struct $name: u64 {
                /// asynchronous read requests
                const ASYNC_READ = 1 << 0;
                /// remote locking for POSIX file locks
                const POSIX_LOCKS = 1 << 1;
                /// kernel sends file handle for fstat, etc. (not yet supported)
                const FILE_OPS = 1 << 2;
                /// handles the O_TRUNC open flag in the filesystem
                const ATOMIC_O_TRUNC = 1 << 3;
                /// filesystem handles lookups of "." and ".."
                const EXPORT_SUPPORT = 1 << 4;
                /// filesystem can handle write size larger than 4kB
                const BIG_WRITES = 1 << 5;
                /// don't apply umask to file mode on create operations
                const DONT_MASK = 1 << 6;
                /// kernel supports splice write on the device
                const SPLICE_WRITE = 1 << 7;
                /// kernel supports splice move on the device
                const SPLICE_MOVE = 1 << 8;
                /// kernel supports splice read on the device
                const SPLICE_READ = 1 << 9;
                /// remote locking for BSD style file locks
                const FLOCK_LOCKS = 1 << 10;
                /// kernel supports ioctl on directories
                const HAS_IOCTL_DIR = 1 << 11;
                /// automatically invalidate cached pages
                const AUTO_INVAL_DATA = 1 << 12;
                /// do READDIRPLUS (READDIR+LOOKUP in one)
                const DO_READDIRPLUS = 1 << 13;
                /// adaptive readdirplus
                const READDIRPLUS_AUTO = 1 << 14;
                /// asynchronous direct I/O submission
                const ASYNC_DIO = 1 << 15;
                /// use writeback cache for buffered writes
                const WRITEBACK_CACHE = 1 << 16;
                /// kernel supports zero-message opens
                const NO_OPEN_SUPPORT = 1 << 17;
                /// allow parallel lookups and readdir
                const PARALLEL_DIROPS = 1 << 18;
                /// fs handles killing suid/sgid/cap on write/chown/trunc
                const HANDLE_KILLPRIV = 1 << 19;
                /// filesystem supports posix acls
                const POSIX_ACL = 1 << 20;
                /// reading the device after abort returns ECONNABORTED
                const ABORT_ERROR = 1 << 21;
                /// init_out.max_pages contains the max number of req pages
                const MAX_PAGES = 1 << 22;
                /// cache READLINK responses
                const CACHE_SYMLINKS = 1 << 23;
                /// kernel supports zero-message opendir
                const NO_OPENDIR_SUPPORT = 1 << 24;
                /// only invalidate cached pages on explicit request
                const EXPLICIT_INVAL_DATA = 1 << 25;
                /// init_out.map_alignment contains log2(byte alignment)
                const MAP_ALIGNMENT = 1 << 26;
                /// kernel supports auto-mounting directory submounts
                const SUBMOUNTS = 1 << 27;
                /// fs kills suid/sgid/cap on write/chown/trunc (v2)
                const HANDLE_KILLPRIV_V2 = 1 << 28;
                /// server supports extended struct fuse_setxattr_in
                const SETXATTR_EXT = 1 << 29;
                /// extended fuse_init_in request
                const INIT_EXT = 1 << 30;
                /// reserved, do not use
                const INIT_RESERVED = 1 << 31;
                /// add security context to create, mkdir, symlink, and mknod
                const SECURITY_CTX = 1 << 32;
                /// use per inode DAX
                const HAS_INODE_DAX = 1 << 33;
                /// add supplementary group info to create, mkdir, symlink and mknod
                const CREATE_SUPP_GROUP = 1 << 34;
                /// kernel supports expiry-only entry invalidation
                const HAS_EXPIRE_ONLY = 1 << 35;
                /// allow shared mmap in FOPEN_DIRECT_IO mode
                const DIRECT_IO_ALLOW_MMAP = 1 << 36;
                /// passthrough of read/write to a backing file
                const PASSTHROUGH = 1 << 37;
                /// explicitly disable export support
                const NO_EXPORT_SUPPORT = 1 << 38;
                /// kernel supports resending pending requests
                const HAS_RESEND = 1 << 39;
                /// allow creation of idmapped mounts
                const ALLOW_IDMAP = 1 << 40;
                /// client supports io-uring
                const OVER_IO_URING = 1 << 41;
                /// kernel supports timing out requests
                const REQUEST_TIMEOUT = 1 << 42;

                /// extended access checks (macOS wire bit 23)
                const DARWIN_ACCESS_EXTENDED = 1 << 55;
                /// per-node reader-writer locks (macOS wire bit 24)
                const DARWIN_NODE_RWLOCK = 1 << 56;
                /// renamex_np(2) RENAME_SWAP (macOS wire bit 25)
                const DARWIN_RENAME_SWAP = 1 << 57;
                /// renamex_np(2) RENAME_EXCL (macOS wire bit 26)
                const DARWIN_RENAME_EXCL = 1 << 58;
                /// fallocate (macOS wire bit 27)
                const DARWIN_ALLOCATE = 1 << 59;
                /// exchangedata(2) (macOS wire bit 28)
                const DARWIN_EXCHANGE_DATA = 1 << 60;
                /// case-insensitive names (macOS wire bit 29)
                const DARWIN_CASE_INSENSITIVE = 1 << 61;
                /// volume renaming (macOS wire bit 30)
                const DARWIN_VOL_RENAME = 1 << 62;
                /// backup and creation times (macOS wire bit 31)
                const DARWIN_XTIMES = 1 << 63;
            }
        }
    )+};
}

init_flags! {
    /// Flags offered by the kernel in `fuse_init_in`.
    struct KernelInitFlags;

    /// Flags sent back to the kernel in `fuse_init_out`.
    struct ReplyInitFlags;
}

impl KernelInitFlags {
    /// Whether `fuse_init_in` carries a valid `flags2` field.
    pub(crate) const fn has_flags2(flags: u32) -> bool {
        !cfg!(target_os = "macos") && flags & Self::INIT_EXT.bits() as u32 != 0
    }

    pub(crate) const fn from_wire(flags: u32, flags2: u32) -> Self {
        if cfg!(target_os = "macos") {
            // Matches the macFUSE library, which decodes bits 23..=31 as Darwin
            // flags regardless of the minor version and never reads `flags2`.
            let linux = (flags & !DARWIN_WIRE_MASK) as u64;
            let darwin = ((flags & DARWIN_WIRE_MASK) as u64) << DARWIN_SHIFT;
            Self::from_bits_truncate(linux | darwin)
        } else {
            let mut bits = flags as u64;
            if Self::has_flags2(flags) {
                bits |= (flags2 as u64) << 32;
            }
            Self::from_bits_truncate(bits & LINUX_MASK)
        }
    }
}

impl ReplyInitFlags {
    /// Builds the INIT reply flags from the filesystem's capabilities.
    ///
    /// `caps` must already be validated against the kernel's offer (see
    /// [`FsCaps::unsupported`]).
    pub(crate) fn negotiate(kernel: KernelInitFlags, caps: FsCaps, request_timeout: bool) -> Self {
        let offered = Self::from_bits_retain(kernel.bits());

        let mut wanted = Self::from_bits_truncate(caps.bits())
            | Self::MAX_PAGES
            | Self::INIT_EXT
            // Always accepted so the macOS rename request layout is unambiguous
            // (see `Rename::decode`); `FsCaps` rename modes are enforced by the
            // library instead.
            | Self::DARWIN_RENAME_SWAP
            | Self::DARWIN_RENAME_EXCL;

        if request_timeout {
            wanted |= Self::REQUEST_TIMEOUT;
        }

        // libfuse always sets BIG_WRITES; `max_write` supersedes it.
        (wanted & offered) | Self::BIG_WRITES
    }

    /// Returns `(flags, flags2)` for `fuse_init_out`.
    pub(crate) const fn to_wire(self) -> (u32, u32) {
        let bits = self.bits();
        if cfg!(target_os = "macos") {
            let linux = bits as u32 & !DARWIN_WIRE_MASK;
            let darwin = (bits >> DARWIN_SHIFT) as u32 & DARWIN_WIRE_MASK;
            (linux | darwin, 0)
        } else {
            let bits = bits & LINUX_MASK;
            (bits as u32, (bits >> 32) as u32)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::KernelCaps;

    fn wire_name(public: &str) -> Option<String> {
        match public {
            "RENAME_WHITEOUT" => None,
            "RENAME_EXCHANGE" => Some("DARWIN_RENAME_SWAP".into()),
            "RENAME_NOREPLACE" => Some("DARWIN_RENAME_EXCL".into()),
            "FALLOCATE" => Some("DARWIN_ALLOCATE".into()),
            "EXCHANGE_DATA" => Some("DARWIN_EXCHANGE_DATA".into()),
            name => Some(
                name.strip_prefix("MACOS_")
                    .map_or_else(|| name.into(), |name| format!("DARWIN_{name}")),
            ),
        }
    }

    #[test]
    fn public_bits_match_wire_layout() {
        for (name, flag) in KernelCaps::all().iter_names() {
            let Some(wire) = wire_name(name) else {
                continue;
            };
            let wire = KernelInitFlags::from_name(&wire)
                .unwrap_or_else(|| panic!("no wire flag for {name}"));
            assert_eq!(flag.bits(), wire.bits(), "{name}");
        }

        for (name, flag) in FsCaps::all().iter_names() {
            let kernel = KernelCaps::from_name(name).map(|flag| flag.bits());
            assert_eq!(kernel, Some(flag.bits()), "{name}");
        }
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn linux_wire_round_trip() {
        let flags = (1 << 0) | (1 << 23) | (1 << 30);
        let flags2 = 1 << 5;
        let kernel = KernelInitFlags::from_wire(flags, flags2);
        assert!(kernel.contains(KernelInitFlags::CACHE_SYMLINKS | KernelInitFlags::PASSTHROUGH));

        let reply = ReplyInitFlags::from_bits_retain(kernel.bits());
        assert_eq!(reply.to_wire(), (flags, flags2));
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn linux_negotiate_drops_darwin_and_unoffered_flags() {
        let kernel = KernelInitFlags::from_wire((1 << 0) | (1 << 22) | (1 << 30), 0);
        let caps = FsCaps::ASYNC_READ | FsCaps::RENAME_EXCHANGE | FsCaps::FALLOCATE;
        let reply = ReplyInitFlags::negotiate(kernel, caps, true);

        assert_eq!(
            reply,
            ReplyInitFlags::ASYNC_READ
                | ReplyInitFlags::MAX_PAGES
                | ReplyInitFlags::INIT_EXT
                | ReplyInitFlags::BIG_WRITES,
        );
    }
}
