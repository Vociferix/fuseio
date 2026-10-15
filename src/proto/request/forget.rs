use super::{Cfg, Ino, RawHeader};
use crate::{Error, Result, buf::Buf};

#[derive(Debug)]
pub struct Forget {
    ino: Ino,
    nlookup: u64,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
struct Raw {
    _hdr: RawHeader,
    nlookup: u64,
}

impl Forget {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn nlookup(&self) -> u64 {
        self.nlookup
    }
}

impl Forget {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some((raw, _)) = buf.as_slice().split_first::<Raw>() else {
            return Err(Error::EPROTO);
        };

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        Ok(Self {
            ino,
            nlookup: raw.nlookup,
        })
    }
}
