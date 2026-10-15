use super::{Cfg, Ino, RawHeader};
use crate::{Error, Result, buf::Buf};

#[derive(Debug)]
pub struct Interrupt {
    id: u64,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct Raw {
    _hdr: RawHeader,
    id: u64,
}

impl Interrupt {
    pub fn id(&self) -> u64 {
        self.id
    }
}

impl Interrupt {
    pub(super) fn decode(buf: Buf, _: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some((raw, _)) = buf.as_slice().split_first::<Raw>() else {
            return Err(Error::EPROTO);
        };

        Ok(Self { id: raw.id })
    }
}
