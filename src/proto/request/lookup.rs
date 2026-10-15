use super::{Cfg, Ino, RawHeader};
use crate::{
    Error, Result,
    buf::{Buf, BufView},
};

use std::ffi::OsStr;

#[derive(Debug)]
pub struct Lookup {
    parent: Ino,
    buf: BufView,
}

impl Lookup {
    pub fn parent(&self) -> Ino {
        self.parent
    }

    pub fn name(&self) -> &OsStr {
        self.buf.as_os_str()
    }
}

impl Lookup {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some((_, name)) = buf.as_slice().split_first::<RawHeader>() else {
            return Err(Error::EPROTO);
        };

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let name_len = memchr::memchr(0, name).unwrap_or_else(|| name.len());

        let Some(buf) = buf.try_into_view(
            std::mem::size_of::<RawHeader>()..(std::mem::size_of::<RawHeader>() + name_len),
        ) else {
            return Err(Error::EPROTO);
        };

        Ok(Self { parent: ino, buf })
    }
}
