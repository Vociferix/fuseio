use super::{Cfg, HDR_LEN, Ino};
use crate::types::FileHandle;
use crate::{Error, Result, buf::Buf};

#[derive(Debug)]
pub struct Fsync {
    ino: Ino,
    fh: FileHandle,
    datasync: bool,
}

#[repr(C)]
struct Raw {
    fh: u64,
    fsync_flags: RawFlags,
    _unused: u32,
}

bitflags::bitflags! {
    #[repr(C)]
    #[derive(Clone, Copy)]
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
            datasync: raw.fsync_flags.contains(RawFlags::DATASYNC),
        })
    }
}
