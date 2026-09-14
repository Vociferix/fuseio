bitflags::bitflags! {
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    pub struct LockFlags: u32 {
        const FLOCK = 1 << 0;
    }
}
