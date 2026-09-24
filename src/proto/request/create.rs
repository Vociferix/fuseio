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
    name_start: usize,
}

#[repr(C)]
struct Raw {
    flags: u32,
    mode: u32,
    umask: u32,
    op_flags: OpenFlags,
}

/// The body before 7.12, which is a `fuse_open_in` carrying the mode in place of
/// its second field, and no umask.
#[repr(C)]
struct RawCompat {
    flags: u32,
    mode: u32,
}

const NAME_OFFSET: usize = HDR_LEN + std::mem::size_of::<Raw>();
const COMPAT_NAME_OFFSET: usize = HDR_LEN + std::mem::size_of::<RawCompat>();

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
        OsStr::from_bytes(&self.buf[self.name_start..])
    }
}

impl Create {
    pub(super) fn decode(mut buf: Buf, ino: Option<Ino>, cfg: Cfg) -> Result<Self> {
        let compat = cfg.minor_ver < 12;
        let name_start = if compat {
            COMPAT_NAME_OFFSET
        } else {
            NAME_OFFSET
        };

        if buf.len() < name_start {
            return Err(Error::EPROTO);
        }

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        // The older body is a prefix of the current one, so the fields they share
        // are read the same way.
        let raw = unsafe { std::ptr::read(buf.as_ptr().add(HDR_LEN) as *const RawCompat) };
        let (umask, op_flags) = if compat {
            (0, OpenFlags::empty())
        } else {
            let raw = unsafe { std::ptr::read(buf.as_ptr().add(HDR_LEN) as *const Raw) };

            (raw.umask, raw.op_flags)
        };

        // TODO(e2e): assumes host-native mode and open flag values; verify once
        // end-to-end tests can be done.
        if SFlag::from_bits_truncate(raw.mode as nix::libc::mode_t) != SFlag::S_IFREG {
            return Err(Error::EINVAL);
        }

        if let Some(name_len) = memchr::memchr(0, &buf[name_start..]) {
            buf.truncate(name_start + name_len);
        }

        Ok(Self {
            parent: ino,
            mode: Mode::from_bits_retain(raw.mode as nix::libc::mode_t),
            umask: Mode::from_bits_retain(umask as nix::libc::mode_t),
            flags: OFlag::from_bits_retain(raw.flags.cast_signed()),
            op_flags,
            buf,
            name_start,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::buf::BufPool;
    use crate::types::ReplyInitFlags;

    const NAME: &[u8] = b"new-file";
    const MODE: u32 = 0o100_644;

    fn cfg(minor_ver: u32) -> Cfg {
        Cfg {
            minor_ver,
            flags: ReplyInitFlags::empty(),
        }
    }

    fn request(umask: Option<u32>) -> Buf {
        let mut buf = BufPool::new().checkout_with_capacity(128);

        buf.extend_from_slice(&[0u8; HDR_LEN]);
        buf.extend_from_slice(&(nix::libc::O_RDWR as u32).to_ne_bytes());
        buf.extend_from_slice(&MODE.to_ne_bytes());

        if let Some(umask) = umask {
            buf.extend_from_slice(&umask.to_ne_bytes());
            buf.extend_from_slice(&0u32.to_ne_bytes()); // open_flags
        }

        buf.extend_from_slice(NAME);
        buf.push(0);

        buf
    }

    #[test]
    fn the_body_decodes() {
        let req = Create::decode(request(Some(0o022)), Ino::from_raw(1), cfg(45)).unwrap();

        assert_eq!(req.name().as_bytes(), NAME);
        assert_eq!(req.mode().bits() as u32, MODE);
        assert_eq!(req.umask().bits() as u32, 0o022);
    }

    // Before 7.12 the body is a `fuse_open_in` and carries no umask.
    #[test]
    fn an_old_kernels_body_decodes() {
        let req = Create::decode(request(None), Ino::from_raw(1), cfg(11)).unwrap();

        assert_eq!(req.name().as_bytes(), NAME);
        assert_eq!(req.mode().bits() as u32, MODE);
        assert!(req.umask().is_empty());
        assert!(req.op_flags().is_empty());
    }

    #[test]
    fn a_short_body_is_rejected() {
        let mut buf = BufPool::new().checkout_with_capacity(64);
        buf.extend_from_slice(&[0u8; HDR_LEN]);
        buf.extend_from_slice(&0u32.to_ne_bytes());

        assert_eq!(
            Create::decode(buf, Ino::from_raw(1), cfg(45)).unwrap_err(),
            Error::EPROTO
        );
    }

    #[test]
    fn a_directory_is_rejected() {
        let mut buf = BufPool::new().checkout_with_capacity(128);
        buf.extend_from_slice(&[0u8; HDR_LEN]);
        buf.extend_from_slice(&0u32.to_ne_bytes());
        buf.extend_from_slice(&0o040_755u32.to_ne_bytes());
        buf.extend_from_slice(&0u32.to_ne_bytes());
        buf.extend_from_slice(&0u32.to_ne_bytes());
        buf.extend_from_slice(NAME);
        buf.push(0);

        assert_eq!(
            Create::decode(buf, Ino::from_raw(1), cfg(45)).unwrap_err(),
            Error::EINVAL
        );
    }
}
