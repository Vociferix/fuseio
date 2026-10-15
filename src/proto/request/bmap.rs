use super::{Cfg, Ino, RawHeader};
use crate::{Error, Result, buf::Buf};

#[derive(Debug)]
pub struct Bmap {
    ino: Ino,
    block: u64,
    block_size: usize,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct Raw {
    hdr: RawHeader,
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
        let Some((raw, _)) = buf.as_slice().split_first::<Raw>() else {
            return Err(Error::EPROTO);
        };

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        Ok(Self {
            ino,
            block: raw.block,
            block_size: raw.block_size as usize,
        })
    }
}
