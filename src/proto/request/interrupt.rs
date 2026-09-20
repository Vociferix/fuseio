use super::{Cfg, HDR_LEN, Ino};
use crate::{Error, Result, buf::Buf};

#[derive(Debug)]
pub struct Interrupt {
    id: u64,
}

impl Interrupt {
    pub fn id(&self) -> u64 {
        self.id
    }
}

impl Interrupt {
    pub(super) fn decode(buf: Buf, _: Option<Ino>, _: Cfg) -> Result<Self> {
        if buf.len() < const { HDR_LEN + std::mem::size_of::<u64>() } {
            return Err(Error::EPROTO);
        }

        let id = unsafe { std::ptr::read(buf.as_ptr().add(HDR_LEN) as *const u64) };

        Ok(Self { id })
    }
}
