use super::{Cfg, HDR_LEN, Ino};
use crate::types::Mode;
use crate::{Buf, Error, Result};

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

#[derive(Debug)]
pub struct MkDir {
    parent: Ino,
    mode: Mode,
    umask: Mode,
    buf: Buf,
}

#[repr(C)]
struct Raw {
    mode: u32,
    umask: u32,
}

const NAME_OFFSET: usize = HDR_LEN + std::mem::size_of::<Raw>();

impl MkDir {
    pub fn parent(&self) -> Ino {
        self.parent
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn umask(&self) -> Mode {
        self.umask
    }

    pub fn name(&self) -> &OsStr {
        OsStr::from_bytes(&self.buf[NAME_OFFSET..])
    }
}

impl MkDir {
    pub(super) fn decode(mut buf: Buf, ino: Option<Ino>, cfg: Cfg) -> Result<Self> {
        if buf.len() < NAME_OFFSET {
            return Err(Error::EPROTO);
        }

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let raw = unsafe { std::ptr::read(buf.as_ptr().add(HDR_LEN) as *const Raw) };
        // TODO(e2e): assumes host-native mode values; verify once end-to-end tests
        // can be done.
        let mode = Mode::from_bits_truncate(raw.mode as nix::libc::mode_t);
        let umask = if cfg.minor_ver < 12 {
            Mode::empty()
        } else {
            Mode::from_bits_truncate(raw.umask as nix::libc::mode_t)
        };

        if let Some(name_len) = memchr::memchr(0, &buf[NAME_OFFSET..]) {
            buf.truncate(name_len + NAME_OFFSET);
        }

        Ok(Self {
            parent: ino,
            mode,
            umask,
            buf,
        })
    }
}
