use super::{Cfg, Ino, RawHeader};
use crate::types::{FallocateFlags, FileHandle};
use crate::{Error, Result, buf::Buf};

#[derive(Debug)]
pub struct Fallocate {
    ino: Ino,
    fh: FileHandle,
    offset: u64,
    len: u64,
    flags: FallocateFlags,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct Raw {
    hdr: RawHeader,
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
        let Some((raw, _)) = buf.as_slice().split_first::<Raw>() else {
            return Err(Error::EPROTO);
        };

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

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
