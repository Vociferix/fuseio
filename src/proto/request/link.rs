use super::{Cfg, HDR_LEN, Ino};
use crate::{Buf, Error, Result};

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

const NAME_OFFSET: usize = HDR_LEN + std::mem::size_of::<u64>();

#[derive(Debug)]
pub struct Link {
    dst: Ino,
    src: Ino,
    buf: Buf,
}

impl Link {
    pub fn dst(&self) -> Ino {
        self.dst
    }

    pub fn src(&self) -> Ino {
        self.src
    }

    pub fn name(&self) -> &OsStr {
        OsStr::from_bytes(&self.buf[NAME_OFFSET..])
    }
}

impl Link {
    pub(super) fn decode(mut buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        if buf.len() < NAME_OFFSET {
            return Err(Error::EPROTO);
        }

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let src = unsafe { std::ptr::read(buf.as_ptr().add(HDR_LEN) as *const u64) };

        let Some(src) = Ino::from_raw(src) else {
            return Err(Error::EINVAL);
        };

        if let Some(name_len) = memchr::memchr(0, &buf[NAME_OFFSET..]) {
            buf.truncate(NAME_OFFSET + name_len);
        }

        Ok(Self { dst: ino, src, buf })
    }
}
