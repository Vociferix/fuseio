use super::{Cfg, Ino};
use crate::{
    Error, Result,
    buf::{Buf, BufView},
};

use std::ffi::OsStr;

#[derive(Debug)]
pub struct GetXattr {
    ino: Ino,
    buf: BufView,
    len: usize,
    #[cfg(target_os = "macos")]
    offset: usize,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
pub(super) struct Raw {
    pub(super) size: u32,
    _unused0: u32,
    #[cfg(target_os = "macos")]
    pub(super) position: u32,
    #[cfg(target_os = "macos")]
    _unused1: u32,
}

impl GetXattr {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn key(&self) -> &OsStr {
        self.buf.as_os_str()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn offset(&self) -> usize {
        #[cfg(target_os = "macos")]
        {
            self.offset
        }

        #[cfg(not(target_os = "macos"))]
        {
            0
        }
    }
}

impl GetXattr {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some((raw, name)) = buf.as_slice().split_first::<Raw>() else {
            return Err(Error::EPROTO);
        };

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let len = raw.size as usize;

        let Some(buf) = (if let Some(key_len) = memchr::memchr(0, name) {
            buf.try_into_view(std::mem::size_of::<Raw>()..(std::mem::size_of::<Raw>() + key_len))
        } else {
            buf.try_into_view(std::mem::size_of::<Raw>()..)
        }) else {
            return Err(Error::EPROTO);
        };

        Ok(Self {
            ino,
            buf,
            len,
            #[cfg(target_os = "macos")]
            offset: raw.position as usize,
        })
    }
}
