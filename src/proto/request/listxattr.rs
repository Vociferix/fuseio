use super::{Cfg, Ino};
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
        let Some((raw, _)) = buf.as_slice().split_first::<Raw>() else {
            return Err(Error::EPROTO);
        };

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        Ok(Self {
            ino,
            len: raw.size as usize,
        })
    }
}
