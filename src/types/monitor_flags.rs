bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    #[repr(C)]
    pub struct MonitorFlags : u32 {
        const BEGIN = 1 << 0;
        const END = 1 << 1;
    }
}
