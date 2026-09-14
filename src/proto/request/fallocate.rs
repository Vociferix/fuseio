use super::{Cfg, HDR_LEN, Ino};
use crate::types::{FallocateFlags, FileHandle};
use crate::{Buf, Error, Result};

#[derive(Debug)]
pub struct Fallocate {
    ino: Ino,
    fh: FileHandle,
    offset: u64,
    len: u64,
    flags: FallocateFlags,
}

#[repr(C)]
struct Raw {
    fh: u64,
    offset: u64,
    len: u64,
    mode: u32,
    _unused: u32,
}

impl Fallocate {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> FileHandle {
        self.fh
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn flags(&self) -> FallocateFlags {
        self.flags
    }
}

impl Fallocate {
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
            offset: raw.offset,
            len: raw.len,
            // TODO(e2e): assumes host-native FALLOC_FL_* values (Linux's where the
            // host has none); verify once end-to-end tests can be done.
            flags: FallocateFlags::from_bits_retain(raw.mode.cast_signed()),
        })
    }
}
