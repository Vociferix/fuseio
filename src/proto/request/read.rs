use super::{Cfg, HDR_LEN, Ino};
use crate::types::{FileHandle, LockOwner, OFlag};
use crate::{Buf, Error, Result};

#[derive(Debug)]
pub struct Read {
    ino: Ino,
    fh: FileHandle,
    offset: u64,
    len: usize,
    lock_owner: Option<LockOwner>,
    flags: OFlag,
}

bitflags::bitflags! {
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Flags: u32 {
        const HAVE_LOCKOWNER = 1 << 1;
    }
}

#[repr(C)]
pub(super) struct Raw {
    fh: u64,
    offset: u64,
    size: u32,
    read_flags: Flags,
    lock_owner: u64,
    flags: u32,
    _unused: u32,
}

impl Read {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> FileHandle {
        self.fh
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn lock_owner(&self) -> Option<LockOwner> {
        self.lock_owner
    }

    pub fn open_flags(&self) -> OFlag {
        self.flags
    }
}

impl Read {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, cfg: Cfg) -> Result<Self> {
        if buf.len() > const { HDR_LEN + std::mem::size_of::<Raw>() } {
            return Err(Error::EPROTO);
        }

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let raw = unsafe { &*(buf.as_ptr().add(HDR_LEN) as *const Raw) };

        let lock_owner = if cfg.minor_ver >= 9 && raw.read_flags.contains(Flags::HAVE_LOCKOWNER) {
            LockOwner::try_from(raw.lock_owner).ok()
        } else {
            None
        };

        Ok(Self {
            ino,
            fh: FileHandle(raw.fh),
            offset: raw.offset,
            len: raw.size as usize,
            lock_owner,
            // TODO(e2e): assumes host-native open flag values; verify once end-to-end
            // tests can be done.
            flags: OFlag::from_bits_retain(raw.flags.cast_signed()),
        })
    }
}
