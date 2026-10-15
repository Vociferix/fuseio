use super::{Cfg, Ino, RawHeader};
use crate::{
    Error, Result,
    buf::{Buf, BufView},
};

use std::ffi::OsStr;

#[derive(Debug)]
pub struct Link {
    parent: Ino,
    ino: Ino,
    buf: BufView,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct Raw {
    hdr: RawHeader,
    src: u64,
}

impl Link {
    /// The directory the new entry goes in.
    pub fn parent(&self) -> Ino {
        self.parent
    }

    /// The file gaining another name.
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn name(&self) -> &OsStr {
        self.buf.as_os_str()
    }
}

impl Link {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some((raw, name)) = buf.as_slice().split_first::<Raw>() else {
            return Err(Error::EPROTO);
        };

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let Some(src) = Ino::from_raw(raw.src) else {
            return Err(Error::EINVAL);
        };

        let Some(buf) = (if let Some(name_len) = memchr::memchr(0, name) {
            buf.try_into_view(std::mem::size_of::<Raw>()..(std::mem::size_of::<Raw>() + name_len))
        } else {
            buf.try_into_view(std::mem::size_of::<Raw>()..)
        }) else {
            return Err(Error::EPROTO);
        };

        Ok(Self {
            parent: ino,
            ino: src,
            buf,
        })
    }
}
