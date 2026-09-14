bitflags::bitflags! {
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    pub struct StatXMask: u32 {
        const TYPE = 1 << 0;
        const MODE = 1 << 1;
        const NLINK = 1 << 2;
        const UID = 1 << 3;
        const GID = 1 << 4;
        const ATIME = 1 << 5;
        const MTIME = 1 << 6;
        const CTIME = 1 << 7;
        const INO = 1 << 8;
        const SIZE = 1 << 9;
        const BLOCKS = 1 << 10;
        const BTIME = 1 << 11;
        const MNT_ID = 1 << 12;
        const DIOALIGN = 1 << 13;
        const MNT_ID_UNIQUE = 1 << 14;
        const SUBVOL = 1 << 15;
        const WRITE_ATOMIC = 1 << 16;
        const DIO_READ_ALIGN = 1 << 17;

        const BASIC_STATS = Self::TYPE.bits()
            | Self::MODE.bits()
            | Self::NLINK.bits()
            | Self::UID.bits()
            | Self::GID.bits()
            | Self::ATIME.bits()
            | Self::MTIME.bits()
            | Self::CTIME.bits()
            | Self::INO.bits()
            | Self::SIZE.bits()
            | Self::BLOCKS.bits();
    }
}
