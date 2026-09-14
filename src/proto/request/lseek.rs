use super::{Cfg, HDR_LEN, Ino};
use crate::types::{FileHandle, Whence};
use crate::{Buf, Error, Result};

#[derive(Debug)]
pub struct Lseek {
    ino: Ino,
    fh: FileHandle,
    offset: u64,
    whence: Whence,
}

#[repr(C)]
struct Raw {
    fh: u64,
    offset: u64,
    whence: u32,
    _unused: u32,
}

impl Lseek {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> FileHandle {
        self.fh
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn whence(&self) -> Whence {
        self.whence
    }
}

impl Lseek {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        if buf.len() < const { HDR_LEN + std::mem::size_of::<Raw>() } {
            return Err(Error::EPROTO);
        }

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let raw = unsafe { &*(buf.as_ptr().add(HDR_LEN) as *const Raw) };

        // TODO(e2e): assumes host-native SEEK_* values; verify once end-to-end
        // tests can be done.
        #[cfg(any(
            target_vendor = "apple",
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "illumos",
            target_os = "solaris",
            target_os = "hurd",
            target_os = "linux",
            target_os = "android"
        ))]
        use nix::libc::{SEEK_DATA, SEEK_HOLE};
        // Hosts without SEEK_DATA/SEEK_HOLE (e.g. NetBSD, whose FUSE predates
        // LSEEK) use Linux's values.
        #[cfg(not(any(
            target_vendor = "apple",
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "illumos",
            target_os = "solaris",
            target_os = "hurd",
            target_os = "linux",
            target_os = "android"
        )))]
        const SEEK_DATA: i32 = 3;
        #[cfg(not(any(
            target_vendor = "apple",
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "illumos",
            target_os = "solaris",
            target_os = "hurd",
            target_os = "linux",
            target_os = "android"
        )))]
        const SEEK_HOLE: i32 = 4;

        let whence = match raw.whence.cast_signed() {
            SEEK_DATA => Whence::Data,
            SEEK_HOLE => Whence::Hole,
            _ => return Err(Error::EINVAL),
        };

        Ok(Self {
            ino,
            fh: FileHandle(raw.fh),
            offset: raw.offset,
            whence,
        })
    }
}
