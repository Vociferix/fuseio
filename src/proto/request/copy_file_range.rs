use super::{Cfg, HDR_LEN, Ino};
use crate::types::{CopyFileRangePos, FileHandle};
use crate::{Buf, Error, Result};

#[derive(Debug)]
pub struct CopyFileRange {
    src: CopyFileRangePos,
    dst: CopyFileRangePos,
    len: u64,
}

#[repr(C)]
struct Raw {
    fh_in: u64,
    off_in: u64,
    ino_out: u64,
    fh_out: u64,
    off_out: u64,
    len: u64,
    flags: u64,
}

impl CopyFileRange {
    pub fn src(&self) -> &CopyFileRangePos {
        &self.src
    }

    pub fn dst(&self) -> &CopyFileRangePos {
        &self.dst
    }

    pub fn len(&self) -> u64 {
        self.len
    }
}

impl CopyFileRange {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        if buf.len() < const { HDR_LEN + std::mem::size_of::<Raw>() } {
            return Err(Error::EPROTO);
        }

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let raw = unsafe { &*(buf.as_ptr().add(HDR_LEN) as *const Raw) };

        let Some(ino_out) = Ino::from_raw(raw.ino_out) else {
            return Err(Error::EINVAL);
        };

        Ok(Self {
            src: CopyFileRangePos {
                ino,
                fh: FileHandle(raw.fh_in),
                offset: raw.off_in,
            },
            dst: CopyFileRangePos {
                ino: ino_out,
                fh: FileHandle(raw.fh_out),
                offset: raw.off_out,
            },
            len: raw.len,
        })
    }
}
