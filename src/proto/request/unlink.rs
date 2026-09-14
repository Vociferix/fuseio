use super::{Cfg, HDR_LEN, Ino};
use crate::{Buf, Error, Result};

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

#[derive(Debug)]
pub struct Unlink {
    ino: Ino,
    buf: Buf,
}

impl Unlink {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn name(&self) -> &OsStr {
        OsStr::from_bytes(&self.buf[HDR_LEN..])
    }
}

impl Unlink {
    pub(super) fn decode(mut buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        if let Some(name_len) = memchr::memchr(0, &buf[HDR_LEN..]) {
            buf.truncate(HDR_LEN + name_len);
        }

        Ok(Self { ino, buf })
    }
}
