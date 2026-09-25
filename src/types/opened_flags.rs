bitflags::bitflags! {
    /// How the kernel should treat a file or directory the filesystem just
    /// opened.
    ///
    /// Every flag is defined on every platform. A kernel ignores the ones it
    /// doesn't know, so the ones marked as belonging to one platform have no
    /// effect elsewhere.
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    pub struct OpenedFlags: u32 {
        /// Reads and writes bypass the page cache.
        const DIRECT_IO = 1 << 0;

        /// Keep the cached contents the kernel already has, rather than
        /// invalidating them on open.
        const KEEP_CACHE = 1 << 1;

        /// The file can't be seeked.
        const NONSEEKABLE = 1 << 2;

        /// Cache this directory's entries.
        const CACHE_DIR = 1 << 3;

        /// The file behaves like a stream, so its offsets carry no meaning
        /// between reads and writes.
        const STREAM = 1 << 4;

        /// Don't send a flush when the file is closed.
        const NO_FLUSH = 1 << 5;

        /// Allow direct writes to the same file to run at the same time.
        const PARALLEL_DIRECT_WRITES = 1 << 6;

        /// Drop the cached attributes, then ask for them again.
        ///
        /// Only effective on macOS, and only together with
        /// [`PURGE_UBC`](Self::PURGE_UBC).
        const PURGE_ATTR = 1 << 30;

        /// Write back and drop everything cached for the file.
        ///
        /// Only effective on macOS.
        const PURGE_UBC = 1 << 31;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flags_match_the_protocol() {
        assert_eq!(OpenedFlags::DIRECT_IO.bits(), 1);
        assert_eq!(OpenedFlags::KEEP_CACHE.bits(), 2);
        assert_eq!(OpenedFlags::PARALLEL_DIRECT_WRITES.bits(), 1 << 6);
        assert_eq!(OpenedFlags::PURGE_ATTR.bits(), 1 << 30);
        assert_eq!(OpenedFlags::PURGE_UBC.bits(), 1 << 31);
    }

    // The passthrough bit the reply builder sets itself.
    #[test]
    fn the_flags_leave_the_passthrough_bit_alone() {
        assert!(!OpenedFlags::all().contains(OpenedFlags::from_bits_retain(1 << 7)));
    }
}
