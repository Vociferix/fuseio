use nix::{request_code_none, request_code_read, request_code_readwrite, request_code_write};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
