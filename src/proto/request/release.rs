use super::{Cfg, HDR_LEN, Ino};
use crate::types::{FileHandle, LockOwner, OFlag};
use crate::{Buf, Error, Result};

#[derive(Debug)]
pub struct Release {
    ino: Ino,
    fh: FileHandle,
    flags: OFlag,
    flush: bool,
    flock_unlock: bool,
    lock_owner: Option<LockOwner>,
}

bitflags::bitflags! {
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct RawFlags: u32 {
        const FLUSH = 1 << 0;
        const FLOCK_UNLOCK = 1 << 1;
    }
}

#[repr(C)]
struct Raw {
    fh: u64,
    flags: u32,
    release_flags: RawFlags,
    lock_owner: u64,
}

impl Release {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> FileHandle {
        self.fh
    }

    pub fn flags(&self) -> OFlag {
        self.flags
    }

    pub fn flush(&self) -> bool {
        self.flush
    }

    pub fn flock_unlock(&self) -> bool {
        self.flock_unlock
    }

    pub fn lock_owner(&self) -> Option<LockOwner> {
        self.lock_owner
    }
}

impl Release {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, cfg: Cfg) -> Result<Self> {
        if buf.len() < const { HDR_LEN + std::mem::size_of::<Raw>() } {
            return Err(Error::EPROTO);
        }

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let raw = unsafe { &*(buf.as_ptr().add(HDR_LEN) as *const Raw) };
        let mut flush = false;
        let mut flock_unlock = false;
        let mut lock_owner = None;

        if cfg.minor_ver >= 8 {
            flush = raw.release_flags.contains(RawFlags::FLUSH);
            lock_owner = LockOwner::try_from(raw.lock_owner).ok();
        }

        if raw.release_flags.contains(RawFlags::FLOCK_UNLOCK) {
            flock_unlock = true;
            lock_owner = LockOwner::try_from(raw.lock_owner).ok();
        }

        Ok(Self {
            ino,
            fh: FileHandle(raw.fh),
            // TODO(e2e): assumes host-native open flag values; verify once end-to-end
            // tests can be done.
            flags: OFlag::from_bits_retain(raw.flags.cast_signed()),
            flush,
            flock_unlock,
            lock_owner,
        })
    }
}
