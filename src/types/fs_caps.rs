use super::KernelCaps;

bitflags::bitflags! {
    /// Features enabled by the filesystem.
    ///
    /// Every flag is defined on every platform. Enabling a flag the kernel
    /// didn't offer (see [`KernelCaps`]) fails the mount, so portable
    /// filesystems should mask their wanted flags with [`FsCaps::supported`].
    ///
    /// Flags that gate an operation (`RENAME_*`, `FALLOCATE`, `EXCHANGE_DATA`)
    /// are enforced by the library on every platform: requests for an operation
    /// that wasn't enabled are rejected without reaching the filesystem, even
    /// where the kernel doesn't negotiate the operation itself.
    // Bit positions match `KernelCaps` and the normalized wire layout in
    // `init_flags.rs`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    pub struct FsCaps: u64 {
        /// The filesystem supports asynchronous read requests.
        ///
        /// Without this flag, the kernel ensures there is at most one pending
        /// read request per file handle, and attempts to order read requests by
        /// increasing offset.
        const ASYNC_READ = 1 << 0;

        /// The filesystem supports "remote" POSIX locking.
        ///
        /// Without this flag, POSIX locks are handled locally by the kernel.
        const POSIX_LOCKS = 1 << 1;

        /// The filesystem handles the `O_TRUNC` open flag.
        ///
        /// Without this flag, the kernel truncates the file with a setattr
        /// request before opening it with `O_TRUNC` filtered out.
        const ATOMIC_O_TRUNC = 1 << 3;

        /// The filesystem supports lookups of "." and "..".
        ///
        /// The filesystem must be prepared to receive requests for invalid
        /// inodes (i.e. inodes that were forgotten, or were used by a previous
        /// instance of the filesystem) and must not reuse inode numbers, even
        /// when setting generation numbers.
        const EXPORT_SUPPORT = 1 << 4;

        /// The kernel should not apply the umask to the file mode on create
        /// operations.
        const DONT_MASK = 1 << 6;

        /// The filesystem supports "remote" BSD locking (`flock(2)`).
        ///
        /// Without this flag, `flock(2)` locks are handled locally by the
        /// kernel, so access that doesn't go through the kernel can't be taken
        /// into account.
        const FLOCK_LOCKS = 1 << 10;

        /// The kernel checks attribute validity on every read.
        ///
        /// Traditionally, while a file is open the kernel only asks the
        /// filesystem for updated attributes when a read extends past EOF,
        /// which is unsuitable for filesystems whose contents may change
        /// without the kernel knowing (e.g. network filesystems). With this
        /// flag, a read issues a getattr request if the cached attributes have
        /// expired, and invalidates cached file contents if the mtime changed.
        ///
        /// If all changes go through the kernel, set a long attribute timeout
        /// to avoid unnecessary getattr requests.
        const AUTO_INVAL_DATA = 1 << 12;

        /// The filesystem supports readdirplus.
        const DO_READDIRPLUS = 1 << 13;

        /// The filesystem supports adaptive readdirplus.
        ///
        /// Has no effect without [`FsCaps::DO_READDIRPLUS`]. With both flags,
        /// the kernel issues readdir or readdirplus requests depending on how
        /// much information it expects to need; with only `DO_READDIRPLUS`, it
        /// always issues readdirplus requests.
        const READDIRPLUS_AUTO = 1 << 14;

        /// The filesystem supports asynchronous direct I/O submission.
        ///
        /// Without this flag, the kernel ensures there is at most one pending
        /// read and one pending write request per direct I/O file handle.
        const ASYNC_DIO = 1 << 15;

        /// The kernel buffers and merges writes before sending them to the
        /// filesystem.
        const WRITEBACK_CACHE = 1 << 16;

        /// The filesystem supports parallel directory operations.
        ///
        /// Without this flag, the kernel never issues lookup and readdir
        /// requests concurrently for the same directory.
        const PARALLEL_DIROPS = 1 << 18;

        /// The filesystem is responsible for clearing the setuid and setgid bits
        /// when a file is written, truncated, or its owner is changed.
        const HANDLE_KILLPRIV = 1 << 19;

        /// The kernel caches and enforces POSIX ACLs.
        ///
        /// ACLs are stored as xattrs and passed to the filesystem, which is
        /// responsible for parsing them, keeping the file mode in sync with the
        /// ACL, and inheriting default ACLs when creating nodes. Implicitly
        /// enables the `default_permissions` mount option.
        const POSIX_ACL = 1 << 20;

        /// The kernel caches symlink targets in its page cache.
        ///
        /// A cached target can be invalidated with an inode invalidation
        /// notification.
        const CACHE_SYMLINKS = 1 << 23;

        /// The filesystem invalidates cached pages only through explicit
        /// invalidation notifications.
        ///
        /// Cached pages may still be flushed by the OS or by user action.
        /// [`FsCaps::AUTO_INVAL_DATA`] takes precedence if both are set.
        const EXPLICIT_INVAL_DATA = 1 << 25;

        /// The filesystem is responsible for clearing the setuid and setgid
        /// bits, and security capabilities stored as xattrs, when a file is
        /// written, truncated, or its owner is changed.
        ///
        /// On write and truncate, setuid/setgid are only cleared if the caller
        /// lacks `CAP_FSETID`, and setgid only if the file is group-executable
        /// (matching the Linux VFS).
        const HANDLE_KILLPRIV_V2 = 1 << 28;

        /// The kernel sends the extended setxattr request, which carries
        /// additional flags (e.g. whether to clear setgid when setting an ACL).
        const SETXATTR_EXT = 1 << 29;

        /// Files opened for direct I/O support shared mmap.
        const DIRECT_IO_ALLOW_MMAP = 1 << 36;

        /// The filesystem may redirect reads and writes to a backing file
        /// handled by the kernel.
        const PASSTHROUGH = 1 << 37;

        /// The filesystem can't be exported over NFS.
        ///
        /// NFS export and `name_to_handle_at(2)` fail with `EOPNOTSUPP`.
        const NO_EXPORT_SUPPORT = 1 << 38;

        /// The filesystem supports renames with `RENAME_WHITEOUT`.
        ///
        /// [`RenameMode::WhiteoutNoReplace`](super::RenameMode::WhiteoutNoReplace)
        /// also requires [`FsCaps::RENAME_NOREPLACE`].
        const RENAME_WHITEOUT = 1 << 50;

        /// The filesystem supports extended access checks.
        ///
        /// Only effective on macOS.
        const MACOS_ACCESS_EXTENDED = 1 << 55;

        /// The filesystem supports per-node reader-writer locking.
        ///
        /// Only effective on macOS.
        const MACOS_NODE_RWLOCK = 1 << 56;

        /// The filesystem supports atomically exchanging two directory entries
        /// (Linux `RENAME_EXCHANGE`, macOS `RENAME_SWAP`).
        const RENAME_EXCHANGE = 1 << 57;

        /// The filesystem supports renames that fail if the target exists
        /// (Linux `RENAME_NOREPLACE`, macOS `RENAME_EXCL`).
        const RENAME_NOREPLACE = 1 << 58;

        /// The filesystem supports preallocating file space.
        const FALLOCATE = 1 << 59;

        /// The filesystem supports `exchangedata(2)`.
        ///
        /// Only offered on macOS. Unlike [`FsCaps::RENAME_EXCHANGE`], file data
        /// moves while object identifiers stay with the paths (see
        /// [`RenameMode::ExchangeData`](super::RenameMode::ExchangeData)).
        const EXCHANGE_DATA = 1 << 60;

        /// The filesystem treats file names as case-insensitive.
        ///
        /// Only effective on macOS.
        const MACOS_CASE_INSENSITIVE = 1 << 61;

        /// The filesystem supports renaming the mounted volume.
        ///
        /// Only effective on macOS.
        const MACOS_VOL_RENAME = 1 << 62;

        /// The filesystem supports backup and creation times.
        ///
        /// Only effective on macOS.
        const MACOS_XTIMES = 1 << 63;
    }
}

impl FsCaps {
    const DEFAULTS: Self = Self::ASYNC_READ
        .union(Self::ATOMIC_O_TRUNC)
        .union(Self::AUTO_INVAL_DATA)
        .union(Self::ASYNC_DIO)
        .union(Self::MACOS_NODE_RWLOCK);

    /// Returns every flag that can be enabled with the given kernel.
    pub const fn supported(kernel: KernelCaps) -> Self {
        Self::from_bits_truncate(kernel.bits())
    }

    /// Returns the flags libfuse enables by default, limited to those offered
    /// by the kernel.
    pub const fn defaults(kernel: KernelCaps) -> Self {
        Self::DEFAULTS.intersection(Self::supported(kernel))
    }

    /// Returns the flags in `self` that the kernel didn't offer.
    pub const fn unsupported(self, kernel: KernelCaps) -> Self {
        self.difference(Self::supported(kernel))
    }
}
