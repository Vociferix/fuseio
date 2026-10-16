use nix::{request_code_none, request_code_read, request_code_readwrite, request_code_write};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, bytemuck::AnyBitPattern)]
#[repr(transparent)]
pub struct IoctlCmd(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IoctlDirection {
    None,
    Read,
    Write,
    ReadWrite,
}

const DIR_NONE: u32 = request_code_none!(0, 0) as u32;
const DIR_READ: u32 = request_code_read!(0, 0, 0) as u32;
const DIR_WRITE: u32 = request_code_write!(0, 0, 0) as u32;
const DIR_READWRITE: u32 = request_code_readwrite!(0, 0, 0) as u32;
const DIR_MASK: u32 = DIR_NONE | DIR_READ | DIR_WRITE | DIR_READWRITE;
const DIR_SHIFT: u32 = DIR_MASK.trailing_zeros();

const GROUP_MASK: u32 = request_code_none!(0xff, 0) as u32 & !DIR_MASK;
const GROUP_SHIFT: u32 = GROUP_MASK.trailing_zeros();

const NUM_MASK: u32 = request_code_none!(0, 0xff) as u32 & !DIR_MASK;
const NUM_SHIFT: u32 = NUM_MASK.trailing_zeros();

const SIZE_SHIFT: u32 = (request_code_read!(0, 0, 1) as u32 & !DIR_MASK).trailing_zeros();
const SIZE_MAX: u32 = {
    let mut shifts = [DIR_SHIFT, GROUP_SHIFT, NUM_SHIFT];

    // 3 element sort
    if shifts[0] > shifts[1] {
        let tmp = shifts[0];
        shifts[0] = shifts[1];
        shifts[1] = tmp;
    }
    if shifts[1] > shifts[2] {
        let tmp = shifts[1];
        shifts[1] = shifts[2];
        shifts[2] = tmp;
    }
    if shifts[0] > shifts[1] {
        let tmp = shifts[0];
        shifts[0] = shifts[1];
        shifts[1] = tmp;
    }

    // get the shift of the subfield ajacent to size in more significant bits, if any
    let next_shift = if SIZE_SHIFT < shifts[0] {
        Some(shifts[0])
    } else if SIZE_SHIFT < shifts[1] {
        Some(shifts[1])
    } else if SIZE_SHIFT < shifts[2] {
        Some(shifts[2])
    } else {
        None
    };

    if let Some(next_shift) = next_shift {
        // if there is a field in more significant bits than size, set all bits below that field and
        // shift down to get just the size field with all bits set (max size)
        ((1u32 << next_shift) - 1) >> SIZE_SHIFT
    } else {
        // if size is the most significant field, set all bits and shift down so only size remains
        u32::MAX >> SIZE_SHIFT
    }
};
const SIZE_MASK: u32 = SIZE_MAX << SIZE_SHIFT;

// Make sure assumptions about ioctl command structure are correct
const _: () = {
    // ensure subfields are just bit packed
    assert!((request_code_read!(1, 0, 0) as u32 & !DIR_MASK).count_ones() == 1);
    assert!((request_code_read!(0, 1, 0) as u32 & !DIR_MASK).count_ones() == 1);
    assert!((request_code_read!(0, 0, 1) as u32 & !DIR_MASK).count_ones() == 1);

    // ensure computed masks have bits set
    assert!(DIR_MASK != 0);
    assert!(GROUP_MASK != 0);
    assert!(NUM_MASK != 0);
    assert!(SIZE_MASK != 0);

    // ensure computed shifts are all distinct
    assert!(DIR_SHIFT != GROUP_SHIFT);
    assert!(DIR_SHIFT != NUM_SHIFT);
    assert!(DIR_SHIFT != SIZE_SHIFT);
    assert!(GROUP_SHIFT != NUM_SHIFT);
    assert!(GROUP_SHIFT != SIZE_SHIFT);
    assert!(NUM_SHIFT != SIZE_SHIFT);
};

impl IoctlCmd {
    pub const MAX_SIZE: usize = SIZE_MAX as usize;

    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn none(group: u8, num: u8) -> Self {
        Self(request_code_none!(group, num) as u32)
    }

    pub const fn read<T>(group: u8, num: u8) -> Self {
        assert!(
            std::mem::size_of::<T>() <= Self::MAX_SIZE,
            "ioctl argument size too large"
        );
        Self(request_code_read!(group, num, std::mem::size_of::<T>()) as u32)
    }

    pub const fn write<T>(group: u8, num: u8) -> Self {
        assert!(
            std::mem::size_of::<T>() <= Self::MAX_SIZE,
            "ioctl argument size too large"
        );
        Self(request_code_write!(group, num, std::mem::size_of::<T>()) as u32)
    }

    pub const fn read_write<T>(group: u8, num: u8) -> Self {
        assert!(
            std::mem::size_of::<T>() <= Self::MAX_SIZE,
            "ioctl argument size too large"
        );
        Self(request_code_readwrite!(group, num, std::mem::size_of::<T>()) as u32)
    }

    pub const fn as_raw(self) -> u32 {
        self.0
    }

    pub const fn direction(self) -> Option<IoctlDirection> {
        match self.0 & DIR_MASK {
            DIR_NONE => Some(IoctlDirection::None),
            DIR_READ => Some(IoctlDirection::Read),
            DIR_WRITE => Some(IoctlDirection::Write),
            DIR_READWRITE => Some(IoctlDirection::ReadWrite),
            _ => None,
        }
    }

    pub const fn is_none(self) -> bool {
        (self.0 & DIR_MASK) == DIR_NONE
    }

    pub const fn is_read(self) -> bool {
        matches!(self.0 & DIR_MASK, DIR_READ | DIR_READWRITE)
    }

    pub const fn is_read_only(self) -> bool {
        (self.0 & DIR_MASK) == DIR_READ
    }

    pub const fn is_write(self) -> bool {
        matches!(self.0 & DIR_MASK, DIR_WRITE | DIR_READWRITE)
    }

    pub const fn is_write_only(self) -> bool {
        (self.0 & DIR_MASK) == DIR_WRITE
    }

    pub const fn is_read_write(self) -> bool {
        (self.0 & DIR_MASK) == DIR_READWRITE
    }

    pub const fn group(self) -> u8 {
        ((self.0 & GROUP_MASK) >> GROUP_SHIFT) as u8
    }

    pub const fn number(self) -> u8 {
        ((self.0 & NUM_MASK) >> NUM_SHIFT) as u8
    }

    pub const fn size(self) -> usize {
        ((self.0 & SIZE_MASK) >> SIZE_SHIFT) as usize
    }
}

impl std::fmt::Debug for IoctlCmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IoctlCmd")
            .field("group", &(*self).group())
            .field("number", &(*self).number())
            .field("direction", &(*self).direction())
            .field("size", &(*self).size())
            .finish()
    }
}

impl From<u32> for IoctlCmd {
    fn from(cmd: u32) -> Self {
        Self(cmd)
    }
}

impl From<IoctlCmd> for u32 {
    fn from(cmd: IoctlCmd) -> Self {
        cmd.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[repr(C)]
    struct Sixteen([u8; 16]);

    // The size field's width is an ABI fact, not a choice: Linux spends 14 bits
    // on it, the BSDs 13 (`IOCPARM_MASK`). Everything else here is derived from
    // the host's own macros, so this is what pins that derivation to reality.
    #[test]
    fn the_size_field_is_as_wide_as_the_host_abi() {
        #[cfg(any(target_os = "linux", target_os = "android"))]
        assert_eq!(IoctlCmd::MAX_SIZE, (1 << 14) - 1);

        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        assert_eq!(IoctlCmd::MAX_SIZE, (1 << 13) - 1);
    }

    // A command is a packed record, and the filesystem reads it back apart.
    // Whatever the host packs, these have to recover.
    #[test]
    fn every_field_survives_the_round_trip() {
        for (group, num) in [(0u8, 0u8), (1, 2), (b'T', 0x1b), (0xff, 0xff)] {
            let none = IoctlCmd::none(group, num);
            assert_eq!(none.group(), group, "group of none({group}, {num})");
            assert_eq!(none.number(), num, "number of none({group}, {num})");
            assert_eq!(none.size(), 0);
            assert_eq!(none.direction(), Some(IoctlDirection::None));

            let read = IoctlCmd::read::<Sixteen>(group, num);
            assert_eq!(read.group(), group);
            assert_eq!(read.number(), num);
            assert_eq!(read.size(), 16);
            assert_eq!(read.direction(), Some(IoctlDirection::Read));

            let write = IoctlCmd::write::<u32>(group, num);
            assert_eq!(write.size(), 4);
            assert_eq!(write.direction(), Some(IoctlDirection::Write));

            let both = IoctlCmd::read_write::<u64>(group, num);
            assert_eq!(both.size(), 8);
            assert_eq!(both.direction(), Some(IoctlDirection::ReadWrite));
        }
    }

    // A raw command off the wire has to decompose the same way one built here
    // does, since that is the only thing connecting the two.
    #[test]
    fn a_raw_command_decomposes_like_a_built_one() {
        let built = IoctlCmd::read_write::<u32>(b'f', 42);
        let raw = IoctlCmd::from_raw(built.as_raw());

        assert_eq!(raw, built);
        assert_eq!(raw.group(), b'f');
        assert_eq!(raw.number(), 42);
        assert_eq!(raw.size(), 4);
        assert_eq!(raw.direction(), Some(IoctlDirection::ReadWrite));
    }

    // The predicates describe one direction between them, and a read-write
    // command is both without being either alone.
    #[test]
    fn the_direction_predicates_agree_with_each_other() {
        let none = IoctlCmd::none(1, 1);
        let read = IoctlCmd::read::<u32>(1, 1);
        let write = IoctlCmd::write::<u32>(1, 1);
        let both = IoctlCmd::read_write::<u32>(1, 1);

        assert!(none.is_none() && !none.is_read() && !none.is_write());

        assert!(read.is_read() && read.is_read_only());
        assert!(!read.is_write() && !read.is_read_write());

        assert!(write.is_write() && write.is_write_only());
        assert!(!write.is_read() && !write.is_read_write());

        assert!(both.is_read() && both.is_write() && both.is_read_write());
        assert!(!both.is_read_only() && !both.is_write_only());
    }

    // Zero-sized arguments are legitimate, and the size field has to say so
    // rather than borrowing a bit from a neighbour.
    #[test]
    fn a_zero_sized_argument_leaves_the_other_fields_alone() {
        let cmd = IoctlCmd::read::<()>(b'x', 7);

        assert_eq!(cmd.size(), 0);
        assert_eq!(cmd.group(), b'x');
        assert_eq!(cmd.number(), 7);
    }

    // Each field occupies its own bits: changing one must not disturb another.
    #[test]
    fn the_fields_do_not_overlap() {
        let a = IoctlCmd::read::<u32>(0, 0);
        let b = IoctlCmd::read::<u32>(0xff, 0);
        let c = IoctlCmd::read::<u32>(0, 0xff);

        assert_eq!((a.number(), a.size()), (b.number(), b.size()));
        assert_eq!((a.group(), a.size()), (c.group(), c.size()));
        assert_ne!(a.as_raw(), b.as_raw());
        assert_ne!(a.as_raw(), c.as_raw());
    }
}
