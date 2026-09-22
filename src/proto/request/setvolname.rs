use super::{Cfg, HDR_LEN, Ino};
use crate::{Result, buf::Buf};

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

#[derive(Debug)]
pub struct SetVolName {
    buf: Buf,
    name_end: usize,
}

impl SetVolName {
    pub fn volume_name(&self) -> &OsStr {
        OsStr::from_bytes(&self.buf[HDR_LEN..self.name_end])
    }
}

impl SetVolName {
    pub(super) fn decode(buf: Buf, _: Option<Ino>, _: Cfg) -> Result<Self> {
        let name_len = memchr::memchr(0, &buf[HDR_LEN..]).unwrap_or(buf.len() - HDR_LEN);
        Ok(Self {
            buf,
            name_end: HDR_LEN + name_len,
        })
    }
}
