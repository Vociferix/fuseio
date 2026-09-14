use super::{Cfg, HDR_LEN, Ino};
use crate::{Buf, Error, Result};

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

#[derive(Debug)]
pub struct Lookup {
    parent: Ino,
    buf: Buf,
}

impl Lookup {
    pub fn parent(&self) -> Ino {
        self.parent
    }

    pub fn name(&self) -> &OsStr {
        OsStr::from_bytes(&self.buf[HDR_LEN..])
    }
}

impl Lookup {
    pub(super) fn decode(mut buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let name_len = memchr::memchr(0, &buf[HDR_LEN..]).unwrap_or_else(|| buf.len() - HDR_LEN);
        buf.truncate(HDR_LEN + name_len);

        Ok(Self { parent: ino, buf })
    }
}
