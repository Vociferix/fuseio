use super::{Cfg, HDR_LEN, Ino};
use crate::types::{DeviceNumber, InodeKind, Mode, SFlag};
use crate::{Error, Result, buf::Buf};

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

#[derive(Debug)]
pub struct MkNod {
    parent: Ino,
    mode: Mode,
    umask: Mode,
    rdev: DeviceNumber,
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

    /// The device the node names, for a character or block device.
    pub fn device_number(&self) -> Option<DeviceNumber> {
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
            let rdev = DeviceNumber::from_raw(hdr.rdev);

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
            let rdev = DeviceNumber::from_raw(hdr.rdev);

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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::buf::BufPool;
    use crate::types::{DeviceNumber, ReplyInitFlags};

    fn cfg(minor_ver: u32) -> Cfg {
        Cfg {
            minor_ver,
            flags: ReplyInitFlags::empty(),
        }
    }

    fn request(mode: u32, rdev: u32, umask: Option<u32>) -> Buf {
        let mut buf = BufPool::new().checkout_with_capacity(128);

        buf.extend_from_slice(&[0u8; HDR_LEN]);
        buf.extend_from_slice(&mode.to_ne_bytes());
        buf.extend_from_slice(&rdev.to_ne_bytes());

        if let Some(umask) = umask {
            buf.extend_from_slice(&umask.to_ne_bytes());
            buf.extend_from_slice(&0u32.to_ne_bytes()); // padding
        }

        buf.extend_from_slice(b"node");
        buf.push(0);

        buf
    }

    fn decode(mode: u32, rdev: u32) -> MkNod {
        MkNod::decode(request(mode, rdev, Some(0o022)), Ino::from_raw(1), cfg(45)).unwrap()
    }

    #[test]
    #[allow(clippy::unnecessary_cast)]
    fn a_character_device_carries_its_numbers() {
        let dev = DeviceNumber::new(4, 65).unwrap();
        let req = decode(0o020_600, dev.as_raw());

        assert_eq!(req.kind(), InodeKind::CharDev);
        assert_eq!(req.device_number(), Some(dev));
        assert_eq!(req.mode().bits() as u32, 0o600);
    }

    #[test]
    fn a_block_device_carries_its_numbers() {
        let dev = DeviceNumber::new(8, 3).unwrap();
        let req = decode(0o060_660, dev.as_raw());

        assert_eq!(req.kind(), InodeKind::BlockDev);
        assert_eq!(req.device_number(), Some(dev));
    }

    // Anything else has no device, whatever the field holds.
    #[test]
    fn a_fifo_has_no_device() {
        let req = decode(0o010_644, 0x803);

        assert_eq!(req.kind(), InodeKind::Fifo);
        assert!(req.device_number().is_none());
    }

    #[test]
    fn a_socket_has_no_device() {
        assert!(decode(0o140_644, 0).device_number().is_none());
    }

    #[test]
    fn an_old_kernels_body_decodes() {
        let buf = request(0o020_600, 0x803, None);
        let req = MkNod::decode(buf, Ino::from_raw(1), cfg(11)).unwrap();

        assert_eq!(req.kind(), InodeKind::CharDev);
        assert!(req.umask().is_empty());
        assert_eq!(req.name().as_bytes(), b"node");
    }

    #[test]
    fn a_directory_is_rejected() {
        let buf = request(0o040_755, 0, Some(0));

        assert_eq!(
            MkNod::decode(buf, Ino::from_raw(1), cfg(45)).unwrap_err(),
            Error::EINVAL
        );
    }
}
