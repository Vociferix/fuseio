use super::{Cfg, EncodeResp, IntoIoBuf, IoBuf, RawHeader};
use crate::types::{DeviceNumber, Gid, Ino, InodeKind, Mode, SFlag, StatXAttrs, StatXMask, Uid};

use std::time::{Duration, SystemTime};

#[repr(C)]
#[derive(Debug)]
pub struct StatX {
    attr_valid: u64,
    attr_valid_nsec: u32,
    flags: u32, // unused for now
    _unused0: [u64; 2],
    mask: StatXMask,
    blksize: u32,
    attributes: StatXAttrs,
    nlink: u32,
    uid: u32,
    gid: u32,
    mode: u16,
    _unused1: u16,
    ino: u64,
    size: u64,
    blocks: u64,
    attributes_mask: StatXAttrs,
    atime: StatXTime,
    btime: StatXTime,
    ctime: StatXTime,
    mtime: StatXTime,
    rdev_major: u32,
    rdev_minor: u32,
    dev_major: u32,
    dev_minor: u32,
    // _unused2: [u64; 14]
}

const _: () = {
    // `fuse_statx_out` without `fuse_statx`'s trailing `__spare2[14]`, which
    // `encode` appends.
    assert!(std::mem::size_of::<StatX>() == 176);
    assert!(std::mem::size_of::<StatXTime>() == 16);
};

#[repr(C)]
#[derive(Debug, Default)]
struct StatXTime {
    secs: i64,
    nsecs: u32,
    _unused: u32,
}

impl StatX {
    pub const fn new() -> Self {
        Self {
            attr_valid: 0,
            attr_valid_nsec: 0,
            flags: 0,
            _unused0: [0; 2],
            mask: StatXMask::empty(),
            blksize: 4096,
            attributes: StatXAttrs::empty(),
            nlink: 1,
            uid: 0,
            gid: 0,
            mode: 0,
            _unused1: 0,
            ino: 0,
            size: 0,
            blocks: 0,
            attributes_mask: StatXAttrs::empty(),
            atime: StatXTime::UNIX_EPOCH,
            btime: StatXTime::UNIX_EPOCH,
            ctime: StatXTime::UNIX_EPOCH,
            mtime: StatXTime::UNIX_EPOCH,
            rdev_major: 0,
            rdev_minor: 0,
            dev_major: 0,
            dev_minor: 0,
            //_unused2: [0; 14]
        }
    }

    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.attr_valid = ttl.as_secs();
        self.attr_valid_nsec = ttl.subsec_nanos();
        self
    }

    pub fn block_size(mut self, size: usize) -> Self {
        self.blksize = u32::try_from(size).unwrap_or(const { 1u32 << 31 });
        self
    }

    pub fn inode_kind(mut self, kind: InodeKind) -> Self {
        self.mask |= StatXMask::TYPE;
        self.mode =
            (SFlag::from(kind).bits() | Mode::from_bits_truncate(self.mode.into()).bits()) as u16;
        self
    }

    pub fn mode(mut self, mode: Mode) -> Self {
        self.mask |= StatXMask::MODE;
        self.mode = (SFlag::from_bits_truncate(self.mode.into()).bits() | mode.bits()) as u16;
        self
    }

    pub fn hard_links(mut self, count: usize) -> Self {
        self.mask |= StatXMask::NLINK;
        self.nlink = u32::try_from(count).unwrap_or(u32::MAX);
        self
    }

    pub fn uid(mut self, uid: Uid) -> Self {
        self.mask |= StatXMask::UID;
        self.uid = uid.as_raw();
        self
    }

    pub fn gid(mut self, gid: Gid) -> Self {
        self.mask |= StatXMask::GID;
        self.gid = gid.as_raw();
        self
    }

    pub fn atime(mut self, time: SystemTime) -> Self {
        self.mask |= StatXMask::ATIME;
        self.atime = time.into();
        self
    }

    pub fn mtime(mut self, time: SystemTime) -> Self {
        self.mask |= StatXMask::MTIME;
        self.mtime = time.into();
        self
    }

    pub fn ctime(mut self, time: SystemTime) -> Self {
        self.mask |= StatXMask::CTIME;
        self.ctime = time.into();
        self
    }

    pub fn btime(mut self, time: SystemTime) -> Self {
        self.mask |= StatXMask::BTIME;
        self.btime = time.into();
        self
    }

    pub fn ino(mut self, ino: Ino) -> Self {
        self.mask |= StatXMask::INO;
        self.ino = ino.as_raw();
        self
    }

    pub fn size(mut self, size: u64) -> Self {
        self.mask |= StatXMask::SIZE;
        self.size = size;
        self
    }

    pub fn blocks(mut self, count: u64) -> Self {
        self.mask |= StatXMask::BLOCKS;
        self.blocks = count;
        self
    }

    /// Sets the device a character or block device names.
    ///
    /// This reply carries the two halves separately, so nothing is packed.
    pub fn inode_device_number(mut self, device: DeviceNumber) -> Self {
        self.rdev_major = device.major();
        self.rdev_minor = device.minor();
        self
    }

    /// Sets the device the filesystem itself lives on.
    pub fn fs_device_number(mut self, device: DeviceNumber) -> Self {
        self.dev_major = device.major();
        self.dev_minor = device.minor();
        self
    }

    pub fn attrs(mut self, attrs: StatXAttrs) -> Self {
        self.attributes = attrs;
        self
    }

    pub fn add_attrs(mut self, attrs: StatXAttrs) -> Self {
        self.attributes |= attrs;
        self
    }

    pub fn supported_attrs(mut self, attrs: StatXAttrs) -> Self {
        self.attributes_mask = attrs;
        self
    }

    pub fn add_supported_attrs(mut self, attrs: StatXAttrs) -> Self {
        self.attributes_mask |= attrs;
        self
    }
}

impl Default for StatX {
    fn default() -> Self {
        const { Self::new() }
    }
}

impl EncodeResp for StatX {
    type Error = std::convert::Infallible;

    fn encode(self, id: u64, _: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            statx: StatX,
            _padding: [u64; 14],
        }

        impl IoBuf for Out {
            fn as_init(&self) -> &[u8] {
                unsafe { std::slice::from_raw_parts(self.buf_ptr(), self.buf_len()) }
            }

            fn buf_ptr(&self) -> *const u8 {
                self as *const Self as *const u8
            }

            fn buf_len(&self) -> usize {
                const { std::mem::size_of::<Out>() }
            }
        }

        // `fuse_out_header` plus `fuse_statx_out`; the kernel rejects any other
        // size.
        const _: () = assert!(std::mem::size_of::<Out>() == 16 + 288);

        Ok(Out {
            hdr: RawHeader {
                len: const { std::mem::size_of::<Out>() as u32 },
                err: 0,
                id,
            },
            statx: self,
            _padding: [0; 14],
        })
    }
}

impl StatXTime {
    const UNIX_EPOCH: Self = Self {
        secs: 0,
        nsecs: 0,
        _unused: 0,
    };
}

impl From<SystemTime> for StatXTime {
    fn from(time: SystemTime) -> Self {
        let (secs, nanos) = crate::proto::time::split(time);

        Self {
            secs,
            nsecs: nanos,
            _unused: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::types::ReplyInitFlags;

    use compio::buf::IoVectoredBuf;

    // Offsets within the reply of the `fuse_statx` fields this exercises.
    const STAT: usize = 16 + 32;
    const MASK: usize = STAT;
    const MODE: usize = STAT + 28;
    const INO: usize = STAT + 32;
    const SIZE: usize = STAT + 40;
    const RDEV_MAJOR: usize = STAT + 128;
    const DEV_MAJOR: usize = STAT + 136;

    fn cfg() -> Cfg {
        Cfg {
            minor_ver: crate::handshake::MINOR_VER,
            flags: ReplyInitFlags::empty(),
        }
    }

    fn encode(statx: StatX) -> Vec<u8> {
        let buf = statx.encode(7, cfg()).unwrap().into_io_buf();

        buf.iter_slice().flatten().copied().collect()
    }

    fn u32_at(bytes: &[u8], offset: usize) -> u32 {
        u32::from_ne_bytes(bytes[offset..offset + 4].try_into().unwrap())
    }

    fn u64_at(bytes: &[u8], offset: usize) -> u64 {
        u64::from_ne_bytes(bytes[offset..offset + 8].try_into().unwrap())
    }

    #[test]
    fn the_reply_is_the_size_the_kernel_expects() {
        let bytes = encode(StatX::new());

        assert_eq!(bytes.len(), 16 + 288);
        assert_eq!(u32_at(&bytes, 0), (16 + 288) as u32);
        assert_eq!(u64_at(&bytes, 8), 7);
    }

    #[test]
    fn the_fields_land_where_the_kernel_reads_them() {
        let ino = Ino::from_raw(42).unwrap();
        let bytes = encode(StatX::new().ino(ino).size(4096).inode_kind(InodeKind::Dir));

        assert_eq!(u64_at(&bytes, INO), 42);
        assert_eq!(u64_at(&bytes, SIZE), 4096);
        assert_eq!(
            u32_at(&bytes, MODE) as u16,
            SFlag::from(InodeKind::Dir).bits() as u16
        );
    }

    #[test]
    fn setters_accumulate_the_mask() {
        let ino = Ino::from_raw(42).unwrap();
        let bytes = encode(StatX::new().ino(ino).size(4096));

        let mask = StatXMask::from_bits_retain(u32_at(&bytes, MASK));

        assert!(mask.contains(StatXMask::INO));
        assert!(mask.contains(StatXMask::SIZE));
        assert!(!mask.contains(StatXMask::NLINK));
    }

    #[test]
    fn inode_and_filesystem_device_numbers_are_separate() {
        let bytes = encode(
            StatX::new()
                .inode_device_number(DeviceNumber::new(1, 2).unwrap())
                .fs_device_number(DeviceNumber::new(3, 4).unwrap()),
        );

        assert_eq!(u32_at(&bytes, RDEV_MAJOR), 1);
        assert_eq!(u32_at(&bytes, RDEV_MAJOR + 4), 2);
        assert_eq!(u32_at(&bytes, DEV_MAJOR), 3);
        assert_eq!(u32_at(&bytes, DEV_MAJOR + 4), 4);
    }
}
