#[cfg(any(
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
#[doc(inline)]
pub use nix::sys::stat::FileFlag;

// Placeholder for platforms without file flags; every flag is empty.
#[cfg(not(any(
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
)))]
bitflags::bitflags! {
    /// File flags.
    pub struct FileFlag: u8 {
        /// The file may only be appended to.
        const SF_APPEND = 0;
        /// The file has been archived.
        const SF_ARCHIVED = 0;
        /// The file may not be changed.
        const SF_IMMUTABLE = 0;
        /// Mask of superuser changeable flags
        const SF_SETTABLE = 0;
        /// The file may only be appended to.
        const UF_APPEND = 0;
        /// File is compressed at the file system level.
        const UF_COMPRESSED = 0;
        /// The file may be hidden from directory listings at the application's
        /// discretion.
        const UF_HIDDEN = 0;
        /// The file may not be changed.
        const UF_IMMUTABLE = 0;
        /// Do not dump the file.
        const UF_NODUMP = 0;
        /// The directory is opaque when viewed through a union stack.
        const UF_OPAQUE = 0;
        /// Mask of owner changeable flags.
        const UF_SETTABLE = 0;
        /// File renames and deletes are tracked.
        const UF_TRACKED = 0;
    }
}
