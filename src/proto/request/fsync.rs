use super::{Cfg, Ino, RawHeader};
use crate::types::FileHandle;
use crate::{Error, Result, buf::Buf};

#[derive(Debug)]
pub struct Fsync {
    ino: Ino,
    fh: FileHandle,
    datasync: bool,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct Raw {
    hdr: RawHeader,
    fh: u64,
    fsync_flags: RawFlags,
    _unused: u32,
}

bitflags::bitflags! {
    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::AnyBitPattern)]
    struct RawFlags: u32 {
        const DATASYNC = 1 << 0;
    }
}

impl Fsync {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> FileHandle {
        self.fh
    }

    pub fn datasync(&self) -> bool {
        self.datasync
    }
}

impl Fsync {
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
            datasync: raw.fsync_flags.contains(RawFlags::DATASYNC),
        })
    }
}
