bitflags::bitflags! {
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    pub struct IoctlFlags: u32 {
        const ABI_32BIT_ON_64BIT = 1 << 0;
        const UNRESTRICTED = 1 << 1;
        const RETRY = 1 << 2;
        const ABI_32BIT = 1 << 3;
        const DIRECTORY = 1 << 4;
        const ABI_X32 = 1 << 5;
    }
}
