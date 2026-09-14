bitflags::bitflags! {
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    pub struct StatXAttrs: u64 {
        const COMPRESSED = 1 << 2;
        const IMMUTABLE = 1 << 4;
        const APPEND = 1 << 5;
        const NODUMP = 1 << 6;
        const ENCRYPTED = 1 << 11;
        const AUTOMOUNT = 1 << 12;
        const MOUNT_ROOT = 1 << 13;
        const VERIFY = 1 << 20;
        const DAX = 1 << 21;
        const ATOMIC_WRITE = 1 << 22;
    }
}
