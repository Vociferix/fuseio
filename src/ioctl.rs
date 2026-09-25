//! The ioctls the FUSE device itself answers.
//!
//! Only Linux has any: FreeBSD's `/dev/fuse` installs no ioctl handler at all,
//! and the macOS kernel extension answers only its own commands.

use nix::sys::ioctl::ioctl_num_type;

/// `FUSE_DEV_IOC_MAGIC`.
const MAGIC: u8 = 229;

/// `struct fuse_backing_map`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BackingMap {
    pub fd: i32,
    pub flags: u32,
    /// The kernel rejects the request unless this is zero.
    pub padding: u64,
}

const _: () = assert!(std::mem::size_of::<BackingMap>() == 16);

/// `FUSE_DEV_IOC_CLONE`.
const CLONE: ioctl_num_type = nix::request_code_read!(MAGIC, 0, std::mem::size_of::<u32>());

/// `FUSE_DEV_IOC_BACKING_OPEN`, which needs Linux 6.9 or newer.
const BACKING_OPEN: ioctl_num_type =
    nix::request_code_write!(MAGIC, 1, std::mem::size_of::<BackingMap>());

/// `FUSE_DEV_IOC_BACKING_CLOSE`.
const BACKING_CLOSE: ioctl_num_type =
    nix::request_code_write!(MAGIC, 2, std::mem::size_of::<u32>());

// Spelled with the request numbers above rather than rebuilt from the parts, so
// the numbers these send are the ones the tests check.
#[cfg(target_os = "linux")]
nix::ioctl_read_bad!(clone_fd, CLONE, u32);

#[cfg(target_os = "linux")]
nix::ioctl_write_ptr_bad!(passthrough_open, BACKING_OPEN, BackingMap);

#[cfg(target_os = "linux")]
nix::ioctl_write_ptr_bad!(passthrough_close, BACKING_CLOSE, u32);

#[cfg(test)]
mod tests {
    use super::*;

    // What the kernel's own macros produce. A wrong magic or sequence number only
    // shows up at runtime as ENOTTY, and for the clone the fallback to `dup`
    // swallows even that, so these are worth pinning exactly.
    #[test]
    fn the_numbers_match_the_kernel() {
        assert_eq!(CLONE, 0x8004_e500, "FUSE_DEV_IOC_CLONE");
        assert_eq!(BACKING_OPEN, 0x4010_e501, "FUSE_DEV_IOC_BACKING_OPEN");
        assert_eq!(BACKING_CLOSE, 0x4004_e502, "FUSE_DEV_IOC_BACKING_CLOSE");
    }

    #[test]
    fn the_magic_fits_the_field_it_goes_in() {
        // A larger magic quietly loses its high bits, which is how these came to
        // be wrong.
        assert_eq!(u32::from(MAGIC) & 0xff, u32::from(MAGIC));
    }

    #[test]
    fn the_backing_map_matches_the_kernels_struct() {
        assert_eq!(size_of::<BackingMap>(), 16);
        assert_eq!(align_of::<BackingMap>(), align_of::<u64>());

        let map = BackingMap {
            fd: 7,
            flags: 0,
            padding: 0,
        };
        let bytes = unsafe {
            std::slice::from_raw_parts(
                (&map as *const BackingMap).cast::<u8>(),
                size_of::<BackingMap>(),
            )
        };

        assert_eq!(i32::from_ne_bytes(bytes[0..4].try_into().unwrap()), 7);
        assert_eq!(u32::from_ne_bytes(bytes[4..8].try_into().unwrap()), 0);
        assert_eq!(u64::from_ne_bytes(bytes[8..16].try_into().unwrap()), 0);
    }
}
