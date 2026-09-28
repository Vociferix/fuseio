use super::KernelInitFlags;

bitflags::bitflags! {
    /// Features offered by the kernel.
    ///
    /// Every flag is defined on every platform; flags a platform's kernel
    /// doesn't implement are never set. Flags that a filesystem can enable have
    /// a counterpart of the same name in [`FsCaps`](super::FsCaps), where they
    /// are documented. The remaining flags are informational only.
    // Bit positions match the normalized wire layout in `init_flags.rs`, so
    // conversions between the types are plain masks.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    pub struct KernelCaps: u64 {
        const ASYNC_READ = 1 << 0;
        const POSIX_LOCKS = 1 << 1;
        const ATOMIC_O_TRUNC = 1 << 3;
        const EXPORT_SUPPORT = 1 << 4;
        const DONT_MASK = 1 << 6;
        const FLOCK_LOCKS = 1 << 10;

        /// The kernel forwards ioctls on directories.
        const HAS_IOCTL_DIR = 1 << 11;

        const AUTO_INVAL_DATA = 1 << 12;
        const DO_READDIRPLUS = 1 << 13;
        const READDIRPLUS_AUTO = 1 << 14;
        const ASYNC_DIO = 1 << 15;
        const WRITEBACK_CACHE = 1 << 16;

        /// The kernel supports zero-message opens.
        ///
        /// Returning `ENOSYS` from `open` is treated as success, and further
        /// opens of the file are handled in the kernel. Without this flag,
        /// `ENOSYS` is returned to the caller as an error.
        const NO_OPEN_SUPPORT = 1 << 17;

        const PARALLEL_DIROPS = 1 << 18;
        const HANDLE_KILLPRIV = 1 << 19;
        const POSIX_ACL = 1 << 20;
        const CACHE_SYMLINKS = 1 << 23;

        /// The kernel supports zero-message opendirs.
        ///
        /// Returning `ENOSYS` from `opendir` is treated as success, and further
        /// opendir and releasedir requests are handled in the kernel. Without
        /// this flag, `ENOSYS` is returned to the caller as an error.
        const NO_OPENDIR_SUPPORT = 1 << 24;

        const EXPLICIT_INVAL_DATA = 1 << 25;
        const HANDLE_KILLPRIV_V2 = 1 << 28;
        const SETXATTR_EXT = 1 << 29;

        /// The kernel supports expiring cached entries without invalidating
        /// them.
        const HAS_EXPIRE_ONLY = 1 << 35;

        const DIRECT_IO_ALLOW_MMAP = 1 << 36;
        const PASSTHROUGH = 1 << 37;
        const NO_EXPORT_SUPPORT = 1 << 38;
        const RENAME_WHITEOUT = 1 << 50;
        const MACOS_ACCESS_EXTENDED = 1 << 55;
        const MACOS_NODE_RWLOCK = 1 << 56;
        const RENAME_EXCHANGE = 1 << 57;
        const RENAME_NOREPLACE = 1 << 58;
        const FALLOCATE = 1 << 59;
        const EXCHANGE_DATA = 1 << 60;
        const MACOS_CASE_INSENSITIVE = 1 << 61;
        const MACOS_VOL_RENAME = 1 << 62;
        const MACOS_XTIMES = 1 << 63;
    }
}

impl KernelCaps {
    pub(crate) const fn new(kernel: KernelInitFlags, minor: u32) -> Self {
        let mut caps = Self::from_bits_truncate(kernel.bits());

        // libfuse infers these from the protocol version rather than trusting
        // the flag bits.
        if minor >= 18 {
            caps = caps.union(Self::HAS_IOCTL_DIR);
        }
        if minor >= 38 {
            caps = caps.union(Self::HAS_EXPIRE_ONLY);
        }

        // macFUSE negotiates these through its own Darwin flags; other kernels
        // may send the corresponding requests whenever the protocol version has
        // them.
        if !cfg!(target_os = "macos") {
            if minor >= 19 {
                caps = caps.union(Self::FALLOCATE);
            }
            if minor >= 23 {
                caps = caps
                    .union(Self::RENAME_EXCHANGE)
                    .union(Self::RENAME_NOREPLACE)
                    .union(Self::RENAME_WHITEOUT);
            }
        }

        caps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INFERRED: KernelCaps = KernelCaps::FALLOCATE
        .union(KernelCaps::RENAME_EXCHANGE)
        .union(KernelCaps::RENAME_NOREPLACE)
        .union(KernelCaps::RENAME_WHITEOUT);

    // Neither Linux nor FreeBSD negotiates these, so the protocol version is all
    // there is to go on. macFUSE negotiates each with a Darwin flag of its own,
    // where assuming them would claim capabilities the kext never offered.
    #[test]
    fn what_is_never_negotiated_is_inferred_off_macos() {
        let caps = KernelCaps::new(KernelInitFlags::empty(), 23);

        assert_eq!(caps.contains(INFERRED), !cfg!(target_os = "macos"));
    }

    #[test]
    fn nothing_is_inferred_below_the_version() {
        let caps = KernelCaps::new(KernelInitFlags::empty(), 18);

        assert!(!caps.intersects(INFERRED));
    }

    // An offer is taken at face value on every platform; only the inference is
    // conditional.
    #[test]
    fn offered_flags_are_always_taken() {
        let offered = KernelInitFlags::DARWIN_ALLOCATE | KernelInitFlags::DARWIN_RENAME_SWAP;

        // A version low enough that nothing is inferred, so only the offer shows.
        let caps = KernelCaps::new(offered, 0);

        assert!(caps.contains(KernelCaps::FALLOCATE | KernelCaps::RENAME_EXCHANGE));
        assert!(!caps.intersects(KernelCaps::RENAME_NOREPLACE | KernelCaps::RENAME_WHITEOUT));
    }

    // No kernel negotiates these either, so they are inferred everywhere.
    #[test]
    fn the_version_only_inferences_apply_everywhere() {
        let caps = KernelCaps::new(KernelInitFlags::empty(), 38);

        assert!(caps.contains(KernelCaps::HAS_IOCTL_DIR | KernelCaps::HAS_EXPIRE_ONLY));
    }
}
