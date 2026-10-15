use super::{Cfg, Ino, RawHeader};
use crate::{
    Error, Result,
    buf::{Buf, BufView},
};

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

#[derive(Debug)]
pub struct Symlink {
    parent: Ino,
    buf: BufView,
    name_len: usize,
}

impl Symlink {
    pub fn parent(&self) -> Ino {
        self.parent
    }

    pub fn name(&self) -> &OsStr {
        OsStr::from_bytes(&self.buf[..self.name_len])
    }

    pub fn link(&self) -> &Path {
        Path::new(OsStr::from_bytes(&self.buf[(self.name_len + 1)..]))
    }
}

impl Symlink {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some((_, data)) = buf.as_slice().split_first::<RawHeader>() else {
            return Err(Error::EPROTO);
        };

        let Some(name_len) = memchr::memchr(0, data) else {
            return Err(Error::EPROTO);
        };

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let Some(buf) = (if let Some(link_len) = memchr::memchr(0, &data[(name_len + 1)..]) {
            buf.try_into_view(
                std::mem::size_of::<RawHeader>()
                    ..(std::mem::size_of::<RawHeader>() + name_len + 1 + link_len),
            )
        } else {
            buf.try_into_view(std::mem::size_of::<RawHeader>()..)
        }) else {
            return Err(Error::EPROTO);
        };

        Ok(Self {
            parent: ino,
            buf,
            name_len,
        })
    }
}
