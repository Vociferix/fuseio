use super::{Cfg, HDR_LEN, Ino};
use crate::{Buf, Error, Result};

#[derive(Debug)]
pub struct Bmap {
    ino: Ino,
    block: u64,
    block_size: usize,
}

#[repr(C)]
struct Raw {
    block: u64,
    block_size: u32,
    _unused: u32,
}

impl Bmap {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn block(&self) -> u64 {
        self.block
    }

    pub fn block_size(&self) -> usize {
        self.block_size
    }
}

impl Bmap {
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
            block: raw.block,
            block_size: raw.block_size as usize,
        })
    }
}
