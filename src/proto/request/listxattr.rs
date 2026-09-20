use super::{Cfg, HDR_LEN, Ino};
use crate::{Error, Result, buf::Buf};

use super::getxattr::Raw;

#[derive(Debug)]
pub struct ListXattr {
    ino: Ino,
    len: usize,
}

impl ListXattr {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

impl ListXattr {
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
            len: raw.size as usize,
        })
    }
}
