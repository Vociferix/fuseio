use super::{Cfg, EncodeResp, IntoIoBuf, IoBuf, RawHeader};
use crate::types::{Gid, Ino, InodeKind, Mode, SFlag, StatXAttrs, StatXMask, Uid};

use std::time::{Duration, SystemTime};

#[repr(C)]
#[derive(Debug)]
pub struct StatX {
    attr_valid: u64,
    attr_valid_nsec: u32,
    flags: u32, // unused for now
    // TODO: `fuse_statx_out.spare` is `uint64_t[2]` (16 bytes), so every field
    // after this is 8 bytes early and the reply is 8 bytes short. Timestamps are
    // `int64_t` and are clamped at the epoch by `From<SystemTime>`.
    _unused0: [u32; 2],
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

#[repr(C)]
#[derive(Debug, Default)]
struct StatXTime {
    secs: u64,
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

    pub fn inode_device_number(mut self, major: u32, minor: u32) -> Self {
        self.rdev_major = major;
        self.rdev_minor = minor;
        self
    }

    // TODO: sets rdev instead of dev_major/dev_minor.
    pub fn fs_device_number(mut self, major: u32, minor: u32) -> Self {
        self.rdev_major = major;
        self.rdev_minor = minor;
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
        let ts = time
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO);
        Self {
            secs: ts.as_secs(),
            nsecs: ts.subsec_nanos(),
            _unused: 0,
        }
    }
}
