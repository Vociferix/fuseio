use super::{Cfg, HDR_LEN, Ino};
use crate::{Buf, Result};

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

#[derive(Debug)]
pub struct SetVolName {
    buf: Buf,
}

impl SetVolName {
    pub fn volume_name(&self) -> &OsStr {
        OsStr::from_bytes(&self.buf[HDR_LEN..])
    }
}

impl SetVolName {
    pub(super) fn decode(buf: Buf, _: Option<Ino>, _: Cfg) -> Result<Self> {
        Ok(Self { buf })
    }
}
