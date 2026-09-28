bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    pub struct NotifyCaps : u32 {
        const INVAL_INODE = 1 << 0;
        const INVAL_ENTRY = 1 << 1;
        const DELETE = 1 << 2;
        const STORE = 1 << 3;
        const RETRIEVE = 1 << 4;
        const POLL_WAKEUP = 1 << 5;
        const EXPIRE_ENTRY = 1 << 6;
        const INC_EPOCH = 1 << 7;
        const PRUNE = 1 << 8;
    }
}
