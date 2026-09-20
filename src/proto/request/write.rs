use super::{Cfg, HDR_LEN, Ino};
use crate::types::{FileHandle, LockOwner, OFlag};
use crate::{Error, Result, buf::Buf};

#[derive(Debug)]
pub struct Write {
    ino: Ino,
    fh: FileHandle,
    offset: u64,
    buf: Buf,
    start: usize,
    lock_owner: Option<LockOwner>,
    flags: OFlag,
    cache_writeback: bool,
    remove_suid_sgid: bool,
}

bitflags::bitflags! {
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct RawFlags: u32 {
        const CACHE = 1 << 0;
        const HAVE_LOCKOWNER = 1 << 1;
        const KILL_SUIDGID = 1 << 2;
    }
}

#[repr(C)]
struct Raw {
    fh: u64,
    offset: u64,
    size: u32,
    write_flags: RawFlags,
    lock_owner: u64,
    flags: u32,
    _unused: u32,
}

#[repr(C)]
struct RawCompat {
    fh: u64,
    offset: u64,
    size: u32,
    write_flags: RawFlags,
}

impl Write {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> FileHandle {
        self.fh
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn data(&self) -> &[u8] {
        &self.buf[self.start..]
    }

    pub fn lock_owner(&self) -> Option<LockOwner> {
        self.lock_owner
    }

    pub fn open_flags(&self) -> OFlag {
        self.flags
    }

    pub fn cache_writeback(&self) -> bool {
        self.cache_writeback
    }

    pub fn remove_suid_sgid(&self) -> bool {
        self.remove_suid_sgid
    }
}

impl Write {
    pub(super) fn decode(mut buf: Buf, ino: Option<Ino>, cfg: Cfg) -> Result<Self> {
        if cfg.minor_ver < 9 {
            const DATA_OFFSET: usize = const { HDR_LEN + std::mem::size_of::<RawCompat>() };

            if buf.len() < DATA_OFFSET {
                return Err(Error::EPROTO);
            }

            let Some(ino) = ino else {
                return Err(Error::EINVAL);
            };

            let raw = unsafe { std::ptr::read(buf.as_ptr().add(HDR_LEN) as *const RawCompat) };

            let len = raw.size as usize;
            buf.truncate(DATA_OFFSET + len);

            Ok(Self {
                ino,
                fh: FileHandle(raw.fh),
                offset: raw.offset,
                buf,
                start: DATA_OFFSET,
                lock_owner: None,
                flags: OFlag::empty(),
                cache_writeback: raw.write_flags.contains(RawFlags::CACHE),
                remove_suid_sgid: raw.write_flags.contains(RawFlags::KILL_SUIDGID),
            })
        } else {
            const DATA_OFFSET: usize = const { HDR_LEN + std::mem::size_of::<Raw>() };

            if buf.len() < DATA_OFFSET {
                return Err(Error::EPROTO);
            }

            let Some(ino) = ino else {
                return Err(Error::EINVAL);
            };

            let raw = unsafe { std::ptr::read(buf.as_ptr().add(HDR_LEN) as *const Raw) };

            let len = raw.size as usize;
            buf.truncate(DATA_OFFSET + len);

            let lock_owner = if raw.write_flags.contains(RawFlags::HAVE_LOCKOWNER) {
                LockOwner::try_from(raw.lock_owner).ok()
            } else {
                None
            };

            Ok(Self {
                ino,
                fh: FileHandle(raw.fh),
                offset: raw.offset,
                buf,
                start: DATA_OFFSET,
                lock_owner,
                // TODO(e2e): assumes host-native open flag values; verify once end-to-end
                // tests can be done.
                flags: OFlag::from_bits_retain(raw.flags.cast_signed()),
                cache_writeback: raw.write_flags.contains(RawFlags::CACHE),
                remove_suid_sgid: raw.write_flags.contains(RawFlags::KILL_SUIDGID),
            })
        }
    }
}
