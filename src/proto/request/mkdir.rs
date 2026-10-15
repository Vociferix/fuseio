use super::{Cfg, Ino, RawHeader};
use crate::types::Mode;
use crate::{
    Error, Result,
    buf::{Buf, BufView},
};

use std::ffi::OsStr;

#[derive(Debug)]
pub struct MkDir {
    parent: Ino,
    mode: Mode,
    umask: Mode,
    buf: BufView,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct Raw {
    hdr: RawHeader,
    mode: u32,
    umask: u32,
}

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
        self.buf.as_os_str()
    }
}

impl MkDir {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, cfg: Cfg) -> Result<Self> {
        let Some((raw, name)) = buf.as_slice().split_first::<Raw>() else {
            return Err(Error::EPROTO);
        };

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        // TODO(e2e): assumes host-native mode values; verify once end-to-end tests
        // can be done.
        let mode = Mode::from_bits_truncate(raw.mode as nix::libc::mode_t);
        let umask = if cfg.minor_ver < 12 {
            Mode::empty()
        } else {
            Mode::from_bits_truncate(raw.umask as nix::libc::mode_t)
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
            mode,
            umask,
            buf,
        })
    }
}
