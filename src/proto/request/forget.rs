use super::{Cfg, HDR_LEN, Ino};
use crate::{Buf, Error, Result};

#[derive(Debug)]
pub struct Forget {
    ino: Ino,
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
        if buf.len() < const { std::mem::size_of::<u64>() + HDR_LEN } {
            return Err(Error::EPROTO);
        }

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let nlookup = unsafe { std::ptr::read(buf.as_ptr().add(HDR_LEN) as *const u64) };

        Ok(Self { ino, nlookup })
    }
}
