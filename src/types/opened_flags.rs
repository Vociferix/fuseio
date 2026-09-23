bitflags::bitflags! {
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    pub struct OpenedFlags: u32 {
        const DIRECT_IO = 1 << 0;
        const KEEP_CACHE = 1 << 1;
        const NONSEEKABLE = 1 << 2;
        const CACHE_DIR = 1 << 3;
        const STREAM = 1 << 4;
        const NO_FLUSH = 1 << 5;
        const PARALLEL_DIRECT_WRITES = 1 << 6;
        // TODO: macOS FOPEN_PURGE_ATTR (1 << 30) and FOPEN_PURGE_UBC (1 << 31)
        // are missing.
    }
}
