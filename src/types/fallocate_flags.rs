// TODO(e2e): assumes each kernel sends host-native values; verify once
// end-to-end tests can be done.
#[cfg(any(target_os = "linux", target_os = "android"))]
use nix::libc as native;

// Only Linux has native fallocate(2) mode flags; Linux's values are used on other
// hosts.
#[cfg(not(any(target_os = "linux", target_os = "android")))]
mod native {
    pub const FALLOC_FL_KEEP_SIZE: i32 = 0x01;
    pub const FALLOC_FL_PUNCH_HOLE: i32 = 0x02;
    pub const FALLOC_FL_COLLAPSE_RANGE: i32 = 0x08;
    pub const FALLOC_FL_ZERO_RANGE: i32 = 0x10;
    pub const FALLOC_FL_INSERT_RANGE: i32 = 0x20;
    pub const FALLOC_FL_UNSHARE_RANGE: i32 = 0x40;
}

bitflags::bitflags! {
    /// Mode flags for a fallocate request.
    ///
    /// Uses the host's `FALLOC_FL_*` values, or Linux's on hosts without them.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    pub struct FallocateFlags: i32 {
        /// File size is not changed.
        ///
        /// `offset + len` can be greater than the file size.
        const FALLOC_FL_KEEP_SIZE = native::FALLOC_FL_KEEP_SIZE;

        /// Deallocates space by creating a hole.
        ///
        /// Must be combined with `FALLOC_FL_KEEP_SIZE`.
        const FALLOC_FL_PUNCH_HOLE = native::FALLOC_FL_PUNCH_HOLE;

        /// Removes a byte range from the file without leaving a hole.
        const FALLOC_FL_COLLAPSE_RANGE = native::FALLOC_FL_COLLAPSE_RANGE;

        /// Zeroes space in a byte range.
        const FALLOC_FL_ZERO_RANGE = native::FALLOC_FL_ZERO_RANGE;

        /// Inserts a hole within the file size without overwriting existing
        /// data.
        const FALLOC_FL_INSERT_RANGE = native::FALLOC_FL_INSERT_RANGE;

        /// Makes shared file data extents private to the file.
        ///
        /// Guarantees that a subsequent write will not fail due to lack of
        /// space.
        const FALLOC_FL_UNSHARE_RANGE = native::FALLOC_FL_UNSHARE_RANGE;
    }
}

#[cfg(target_os = "linux")]
impl From<FallocateFlags> for nix::fcntl::FallocateFlags {
    fn from(flags: FallocateFlags) -> Self {
        Self::from_bits_retain(flags.bits())
    }
}

#[cfg(target_os = "linux")]
impl From<nix::fcntl::FallocateFlags> for FallocateFlags {
    fn from(flags: nix::fcntl::FallocateFlags) -> Self {
        Self::from_bits_retain(flags.bits())
    }
}
