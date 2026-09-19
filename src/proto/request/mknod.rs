use super::{Cfg, HDR_LEN, Ino};
use crate::types::{InodeKind, Mode, SFlag};
use crate::{Buf, Error, Result};

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

#[derive(Debug)]
pub struct MkNod {
    parent: Ino,
    mode: Mode,
    umask: Mode,
    rdev: u32,
    buf: Buf,
    name_start: usize,
}

impl MkNod {
    pub fn parent(&self) -> Ino {
        self.parent
    }

    pub fn mode(&self) -> Mode {
        self.mode & Mode::all()
    }

    pub fn umask(&self) -> Mode {
        self.umask
    }

    pub fn kind(&self) -> InodeKind {
        match SFlag::from_bits_truncate(self.mode.bits()) {
            SFlag::S_IFREG => InodeKind::File,
            SFlag::S_IFDIR => InodeKind::Dir,
            SFlag::S_IFCHR => InodeKind::CharDev,
            SFlag::S_IFBLK => InodeKind::BlockDev,
            SFlag::S_IFIFO => InodeKind::Fifo,
            SFlag::S_IFSOCK => InodeKind::Socket,
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    pub fn rdev(&self) -> Option<u32> {
        matches!(self.kind(), InodeKind::CharDev | InodeKind::BlockDev).then_some(self.rdev)
    }

    pub fn name(&self) -> &OsStr {
        OsStr::from_bytes(&self.buf[self.name_start..])
    }
}

#[repr(C)]
struct RawCompat {
    mode: u32,
    rdev: u32,
}

#[repr(C)]
struct Raw {
    mode: u32,
    rdev: u32,
    umask: u32,
    _unused: u32,
}

impl MkNod {
    pub(super) fn decode(mut buf: Buf, ino: Option<Ino>, cfg: Cfg) -> Result<Self> {
        if cfg.minor_ver < 12 {
            const NAME_OFFSET: usize = HDR_LEN + std::mem::size_of::<RawCompat>();

            if buf.len() < NAME_OFFSET {
                return Err(Error::EPROTO);
            }

            let Some(ino) = ino else {
                return Err(Error::EINVAL);
            };

            let hdr = unsafe { std::ptr::read(buf.as_ptr().add(HDR_LEN) as *const RawCompat) };

            // TODO(e2e): assumes host-native mode values; verify once end-to-end tests
            // can be done.
            let mode = Mode::from_bits_retain(hdr.mode as nix::libc::mode_t);
            let rdev = hdr.rdev;

            if !matches!(
                SFlag::from_bits_truncate(hdr.mode as nix::libc::mode_t),
                SFlag::S_IFSOCK | SFlag::S_IFIFO | SFlag::S_IFBLK | SFlag::S_IFCHR | SFlag::S_IFREG
            ) {
                return Err(Error::EINVAL);
            }

            let name_len =
                memchr::memchr(0, &buf[NAME_OFFSET..]).unwrap_or_else(|| buf.len() - NAME_OFFSET);
            buf.truncate(NAME_OFFSET + name_len);

            Ok(Self {
                parent: ino,
                mode,
                umask: Mode::empty(),
                rdev,
                buf,
                name_start: NAME_OFFSET,
            })
        } else {
            const NAME_OFFSET: usize = HDR_LEN + std::mem::size_of::<Raw>();

            if buf.len() < NAME_OFFSET {
                return Err(Error::EPROTO);
            }

            let Some(ino) = ino else {
                return Err(Error::EINVAL);
            };

            let hdr = unsafe { std::ptr::read(buf.as_ptr().add(HDR_LEN) as *const Raw) };

            // TODO(e2e): assumes host-native mode values; verify once end-to-end tests
            // can be done.
            let mode = Mode::from_bits_retain(hdr.mode as nix::libc::mode_t);
            let umask = Mode::from_bits_truncate(hdr.umask as nix::libc::mode_t);
            let rdev = hdr.rdev;

            if !matches!(
                SFlag::from_bits_truncate(mode.bits()),
                SFlag::S_IFSOCK | SFlag::S_IFIFO | SFlag::S_IFBLK | SFlag::S_IFCHR | SFlag::S_IFREG
            ) {
                return Err(Error::EINVAL);
            }

            let name_len =
                memchr::memchr(0, &buf[NAME_OFFSET..]).unwrap_or_else(|| buf.len() - NAME_OFFSET);
            buf.truncate(NAME_OFFSET + name_len);

            Ok(Self {
                parent: ino,
                mode,
                umask,
                rdev,
                buf,
                name_start: NAME_OFFSET,
            })
        }
    }
}
