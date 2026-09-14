use super::{Cfg, HDR_LEN, Ino};
use crate::types::{FileHandle, LockOwner};
use crate::{Buf, Error, Result};

#[derive(Debug)]
pub struct Flush {
    ino: Ino,
    fh: FileHandle,
    lock_owner: Option<LockOwner>,
}

#[repr(C)]
struct Raw {
    fh: u64,
    _unused0: u32,
    _unused1: u32,
    lock_owner: u64,
}

impl Flush {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> FileHandle {
        self.fh
    }

    pub fn lock_owner(&self) -> Option<LockOwner> {
        self.lock_owner
    }
}

impl Flush {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        if buf.len() < const { HDR_LEN + std::mem::size_of::<Raw>() } {
            return Err(Error::EPROTO);
        }

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let raw = unsafe { &*(buf.as_ptr().add(HDR_LEN) as *const Raw) };

        Ok(Self {
            ino,
            fh: FileHandle(raw.fh),
            lock_owner: LockOwner::try_from(raw.lock_owner).ok(),
        })
    }
}
