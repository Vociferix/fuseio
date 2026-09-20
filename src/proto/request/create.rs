use super::{Cfg, HDR_LEN, Ino};
use crate::types::{Mode, OFlag, OpenFlags, SFlag};
use crate::{Error, Result, buf::Buf};

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

#[derive(Debug)]
pub struct Create {
    parent: Ino,
    mode: Mode,
    umask: Mode,
    flags: OFlag,
    op_flags: OpenFlags,
    buf: Buf,
}

#[repr(C)]
struct Raw {
    flags: u32,
    mode: u32,
    umask: u32,
    op_flags: OpenFlags,
}

const NAME_OFFSET: usize = HDR_LEN + std::mem::size_of::<Raw>();

impl Create {
    pub fn parent(&self) -> Ino {
        self.parent
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn umask(&self) -> Mode {
        self.umask
    }

    pub fn flags(&self) -> OFlag {
        self.flags
    }

    pub fn op_flags(&self) -> OpenFlags {
        self.op_flags
    }

    pub fn name(&self) -> &OsStr {
        OsStr::from_bytes(&self.buf[NAME_OFFSET..])
    }
}

impl Create {
    pub(super) fn decode(mut buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        if buf.len() < NAME_OFFSET {
            return Err(Error::EPROTO);
        }

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let raw = unsafe { std::ptr::read(buf.as_ptr().add(HDR_LEN) as *const Raw) };

        // TODO(e2e): assumes host-native mode and open flag values; verify once
        // end-to-end tests can be done.
        if SFlag::from_bits_truncate(raw.mode as nix::libc::mode_t) != SFlag::S_IFREG {
            return Err(Error::EINVAL);
        }

        if let Some(name_len) = memchr::memchr(0, &buf[NAME_OFFSET..]) {
            buf.truncate(NAME_OFFSET + name_len);
        }

        Ok(Self {
            parent: ino,
            mode: Mode::from_bits_retain(raw.mode as nix::libc::mode_t),
            umask: Mode::from_bits_retain(raw.umask as nix::libc::mode_t),
            flags: OFlag::from_bits_retain(raw.flags.cast_signed()),
            op_flags: raw.op_flags,
            buf,
        })
    }
}
