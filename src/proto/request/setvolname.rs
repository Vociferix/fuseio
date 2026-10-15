use super::{Cfg, Ino, RawHeader};
use crate::{
    Error, Result,
    buf::{Buf, BufView},
};

use std::ffi::OsStr;

#[derive(Debug)]
pub struct SetVolName {
    buf: BufView,
}

impl SetVolName {
    pub fn volume_name(&self) -> &OsStr {
        self.buf.as_os_str()
    }
}

impl SetVolName {
    pub(super) fn decode(buf: Buf, _: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some((_, name)) = buf.as_slice().split_first::<RawHeader>() else {
            return Err(Error::EPROTO);
        };

        let Some(buf) = (if let Some(name_len) = memchr::memchr(0, name) {
            buf.try_into_view(
                std::mem::size_of::<RawHeader>()..(std::mem::size_of::<RawHeader>() + name_len),
            )
        } else {
            buf.try_into_view(std::mem::size_of::<RawHeader>()..)
        }) else {
            return Err(Error::EPROTO);
        };

        Ok(Self { buf })
    }
}
