use crate::request::Ino;

use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version(pub u32, pub u32);

pub struct KernelConfig {
    version: Version,
    max_readahead: u32,
    flags: InitFlags,
}

pub struct FsConfig {
    k_max_readahead: u32,
    max_readahead: u32,
    pub(crate) flags: InitFlags,
    max_background: u16,
    congestion_threshold: Option<u16>,
    max_write: u32,
    time_gran: Duration,
    passthrough: bool,
    stackable: bool,
    pub(crate) root_inode: Ino,
}

const MACOS: bool = cfg!(target_os = "macos");

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct InitFlags: u64 {
        /// asynchronous read requests
        const ASYNC_READ = 1 << 0;
        /// remote locking for POSIX file locks
        const POSIX_LOCKS = 1 << 1;
        /// kernel sends file handle for fstat, etc...
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
        /// map_alignment field is valid
        const MAP_ALIGNMENT = 1 << 26;
        /// filesystem supports submounts
        const SUBMOUNTS = 1 << (if MACOS { 59 } else { 27 });
        /// fs handles killing suid/sgid/cap on write/chown/trunc (v2)
        const HANDLE_KILLPRIV_V2 = 1 << (if MACOS { 60 } else { 28 });
        /// extended setxattr support
        const SETXATTR_EXT = 1 << (if MACOS { 61 } else { 29 });
        /// extended fuse_init_in request
        const INIT_EXT = 1 << (if MACOS { 62 } else { 30 });
        /// reserved, do not use
        const INIT_RESERVED = 1 << (if MACOS { 63 } else { 31 });
        /// add security context to create/mkdir/symlink/mknod
        const SECURITY_CTX = 1 << 32;
        /// filesystem supports per-inode DAX
        const HAS_INODE_DAX = 1 << 33;
        /// create with supplementary group
        const CREATE_SUPP_GROUP = 1 << 34;
        /// kernel supports expire-only invalidation
        const HAS_EXPIRE_ONLY = 1 << 35;
        /// allow mmap for direct I/O files
        const DIRECT_IO_ALLOW_MMAP = 1 << 36;
        /// filesystem wants to use passthrough files
        const PASSTHROUGH = 1 << 37;
        /// filesystem does not support export
        const NO_EXPORT_SUPPORT = 1 << 38;
        /// kernel supports resend requests
        const HAS_RESEND = 1 << 39;
        /// allow idmapped mounts
        const ALLOW_IDMAP = 1 << 40;
        /// kernel supports io_uring for communication
        const OVER_IO_URING = 1 << 41;
        /// kernel supports request timeout
        const REQUEST_TIMEOUT = 1 << 42;

        // macos-only flags

        /// pre-allocate space for a file
        const ALLOCATE = 1 << (if MACOS { 27 } else { 59 });
        /// atomically exchange data between files
        const EXCHANGE_DATA = 1 << (if MACOS { 28 } else { 60 });
        /// filesystem is case-insensitive
        const CASE_INSENSITIVE = 1 << (if MACOS { 29 } else { 61 });
        /// filesystem supports volume renaming
        const VOL_RENAME = 1 << (if MACOS { 30 } else { 62 });
        /// filesystem supports extended times (backup and creation times)
        const XTIMES = 1 << (if MACOS { 31 } else { 63 });
    }
}

impl KernelConfig {
    pub(crate) fn new(init: crate::layout::InitIn) -> Self {
        Self {
            version: Version(init.major, init.minor),
            max_readahead: init.max_readahead,
            flags: InitFlags::from_bits_retain(
                ((u64::from(init.flags2) as u64) << 32) | u64::from(init.flags),
            ),
        }
    }

    pub fn version(&self) -> Version {
        self.version
    }

    pub fn flags(&self) -> InitFlags {
        self.flags
    }

    pub fn supports(&self, flags: InitFlags) -> bool {
        self.flags.contains(flags)
    }

    pub fn max_readahead(&self) -> usize {
        self.max_readahead as usize
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.0, self.1)
    }
}

impl FsConfig {
    pub const fn new(kconf: KernelConfig) -> Self {
        Self {
            k_max_readahead: kconf.max_readahead,
            max_readahead: kconf.max_readahead,
            #[cfg(not(target_os = "macos"))]
            flags: InitFlags::ASYNC_READ
                .union(InitFlags::BIG_WRITES)
                .union(kconf.flags.intersection(InitFlags::MAX_PAGES)),
            #[cfg(target_os = "macos")]
            flags: InitFlags::ASYNC_READ
                .union(InitFlags::CASE_INSENSITIVE)
                .union(InitFlags::VOL_RENAME)
                .union(InitFlags::XTIMES)
                .union(kconf.flags.intersection(InitFlags::MAX_PAGES)),
            max_background: 64,
            congestion_threshold: None,
            max_write: crate::MAX_WRITE_SIZE as u32,
            time_gran: Duration::from_nanos(1),
            passthrough: false,
            stackable: false,
            root_inode: unsafe { Ino::from_raw_unchecked(1) },
        }
    }

    pub fn max_readahead(mut self, max: usize) -> Self {
        self.max_readahead = u32::try_from(max)
            .unwrap_or(u32::MAX)
            .min(self.k_max_readahead);
        self
    }

    pub fn enable(mut self, flags: InitFlags) -> Self {
        self.flags |= flags;
        self
    }

    pub fn disable(mut self, flags: InitFlags) -> Self {
        self.flags &= !flags;
        self
    }

    pub fn max_background_reqs(mut self, max: u16) -> Self {
        self.max_background = max.max(1);
        self
    }

    pub fn congestion_threshold(mut self, threshold: impl Into<Option<u16>>) -> Self {
        self.congestion_threshold = threshold.into().map(|thr| thr.max(1));
        self
    }

    pub fn max_write_size(mut self, max: usize) -> Self {
        self.max_write = u32::try_from(max)
            .unwrap_or(const { crate::MAX_WRITE_SIZE as u32 })
            .min(const { crate::MAX_WRITE_SIZE as u32 });
        self
    }

    pub fn time_granularity(mut self, granularity: Duration) -> Self {
        self.time_gran = granularity;
        self
    }

    pub fn with_passthrough(mut self, enable: bool) -> Self {
        self.passthrough = enable;
        self
    }

    pub fn stackable(mut self, enable: bool) -> Self {
        self.stackable = enable;
        self
    }

    pub fn root_ino(mut self, ino: Ino) -> Self {
        self.root_inode = ino;
        self
    }

    pub(crate) fn build(self) -> crate::layout::InitOut {
        let flags = self.flags.bits() as u32;
        let flags2 = (self.flags.bits() >> 32) as u32;

        let congestion_threshold = self.congestion_threshold.unwrap_or_else(|| {
            u16::try_from(self.max_background as u32 * 4 / 3).unwrap_or(u16::MAX)
        });

        let max_pages =
            ((self.max_write.max(self.max_readahead) - 1) / page_size::get() as u32) as u16 + 1;

        let max_stack_depth = if self.stackable {
            2
        } else if self.passthrough {
            1
        } else {
            0
        };

        crate::layout::InitOut {
            major: crate::layout::VERSION_MAJOR,
            minor: crate::layout::VERSION_MINOR,
            max_readahead: self.max_readahead,
            flags,
            max_background: self.max_background,
            congestion_threshold,
            max_write: self.max_write,
            time_gran: u32::try_from(self.time_gran.as_nanos()).unwrap_or(u32::MAX),
            max_pages,
            _unused: 0,
            flags2,
            max_stack_depth,
            _reserved: [0u32; 6],
        }
    }
}

impl From<KernelConfig> for FsConfig {
    fn from(kconf: KernelConfig) -> Self {
        Self::new(kconf)
    }
}
