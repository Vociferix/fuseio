use super::{Cfg, HDR_LEN, Ino};
use crate::{Error, Result, buf::Buf};

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

#[derive(Debug)]
pub struct RemoveXattr {
    ino: Ino,
    buf: Buf,
}

impl RemoveXattr {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn key(&self) -> &OsStr {
        OsStr::from_bytes(&self.buf[HDR_LEN..])
    }
}

impl RemoveXattr {
    pub(super) fn decode(mut buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        if let Some(key_len) = memchr::memchr(0, &buf[HDR_LEN..]) {
            buf.truncate(HDR_LEN + key_len);
        }

        Ok(Self { ino, buf })
    }
}
