use super::{Cfg, HDR_LEN, Ino};
use crate::{Error, Result, buf::Buf};

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

#[derive(Debug)]
pub struct GetXattr {
    ino: Ino,
    buf: Buf,
    len: usize,
    #[cfg(target_os = "macos")]
    offset: usize,
}

#[repr(C)]
pub(super) struct Raw {
    pub(super) size: u32,
    _unused0: u32,
    #[cfg(target_os = "macos")]
    pub(super) position: u32,
    #[cfg(target_os = "macos")]
    _unused1: u32,
}

const KEY_OFFSET: usize = HDR_LEN + std::mem::size_of::<Raw>();

impl GetXattr {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn key(&self) -> &OsStr {
        OsStr::from_bytes(&self.buf[KEY_OFFSET..])
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
    pub(super) fn decode(mut buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        if buf.len() < KEY_OFFSET {
            return Err(Error::EPROTO);
        }

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let raw = unsafe { std::ptr::read(buf.as_ptr().add(HDR_LEN) as *const Raw) };

        if let Some(key_len) = memchr::memchr(0, &buf[KEY_OFFSET..]) {
            buf.truncate(KEY_OFFSET + key_len);
        }

        Ok(Self {
            ino,
            buf,
            len: raw.size as usize,
            #[cfg(target_os = "macos")]
            offset: raw.position as usize,
        })
    }
}
