bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct OpenFlags: u32 {
        const KILL_SUIDGID = 1 << 0;
    }
}
