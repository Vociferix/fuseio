use super::{Cfg, HDR_LEN, Ino};
use crate::{Error, Result, buf::Buf};

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

#[derive(Debug)]
pub struct Symlink {
    parent: Ino,
    buf: Buf,
    name_len: usize,
}

impl Symlink {
    pub fn parent(&self) -> Ino {
        self.parent
    }

    pub fn name(&self) -> &OsStr {
        OsStr::from_bytes(&self.buf[HDR_LEN..(HDR_LEN + self.name_len)])
    }

    pub fn link(&self) -> &Path {
        Path::new(OsStr::from_bytes(
            &self.buf[(self.name_len + const { HDR_LEN + 1 })..],
        ))
    }
}

impl Symlink {
    pub(super) fn decode(mut buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some(name_len) = memchr::memchr(0, &buf[HDR_LEN..]) else {
            return Err(Error::EPROTO);
        };

        if buf.len() < HDR_LEN + name_len + 2 {
            return Err(Error::EPROTO);
        }

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        if let Some(link_len) = memchr::memchr(0, &buf[(HDR_LEN + name_len + 1)..]) {
            buf.truncate(HDR_LEN + name_len + link_len + 1);
        }

        Ok(Self {
            parent: ino,
            buf,
            name_len,
        })
    }
}
