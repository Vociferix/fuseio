use super::{Cfg, Ino, RawHeader};
use crate::types::{DeviceNumber, InodeKind, Mode, SFlag};
use crate::{
    Error, Result,
    buf::{Buf, BufView},
};

use std::ffi::OsStr;

#[derive(Debug)]
pub struct MkNod {
    parent: Ino,
    mode: Mode,
    umask: Mode,
    rdev: DeviceNumber,
    buf: BufView,
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
            _ => InodeKind::Socket,
        }
    }

    /// The device the node names, for a character or block device.
    pub fn device_number(&self) -> Option<DeviceNumber> {
        matches!(self.kind(), InodeKind::CharDev | InodeKind::BlockDev).then_some(self.rdev)
    }

    pub fn name(&self) -> &OsStr {
        self.buf.as_os_str()
    }
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct RawCompat {
    hdr: RawHeader,
    mode: u32,
    rdev: u32,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct Raw {
    hdr: RawHeader,
    mode: u32,
    rdev: u32,
    umask: u32,
    _unused: u32,
}

impl MkNod {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, cfg: Cfg) -> Result<Self> {
        if cfg.minor_ver < 12 {
            let Some((raw, name)) = buf.as_slice().split_first::<RawCompat>() else {
                return Err(Error::EPROTO);
            };

            let Some(ino) = ino else {
                return Err(Error::EINVAL);
            };

            // TODO(e2e): assumes host-native mode values; verify once end-to-end tests
            // can be done.
            let mode = Mode::from_bits_retain(raw.mode as nix::libc::mode_t);
            let rdev = DeviceNumber::from_raw(raw.rdev);

            if !matches!(
                SFlag::from_bits_truncate(raw.mode as nix::libc::mode_t),
                SFlag::S_IFSOCK | SFlag::S_IFIFO | SFlag::S_IFBLK | SFlag::S_IFCHR | SFlag::S_IFREG
            ) {
                return Err(Error::EINVAL);
            }

            let Some(buf) = (if let Some(name_len) = memchr::memchr(0, name) {
                buf.try_into_view(
                    std::mem::size_of::<RawCompat>()..(std::mem::size_of::<RawCompat>() + name_len),
                )
            } else {
                buf.try_into_view(std::mem::size_of::<RawCompat>()..)
            }) else {
                return Err(Error::EPROTO);
            };

            Ok(Self {
                parent: ino,
                mode,
                umask: Mode::empty(),
                rdev,
                buf,
            })
        } else {
            let Some((raw, name)) = buf.as_slice().split_first::<Raw>() else {
                return Err(Error::EPROTO);
            };

            let Some(ino) = ino else {
                return Err(Error::EINVAL);
            };

            // TODO(e2e): assumes host-native mode values; verify once end-to-end tests
            // can be done.
            let mode = Mode::from_bits_retain(raw.mode as nix::libc::mode_t);
            let umask = Mode::from_bits_truncate(raw.umask as nix::libc::mode_t);
            let rdev = DeviceNumber::from_raw(raw.rdev);

            if !matches!(
                SFlag::from_bits_truncate(mode.bits()),
                SFlag::S_IFSOCK | SFlag::S_IFIFO | SFlag::S_IFBLK | SFlag::S_IFCHR | SFlag::S_IFREG
            ) {
                return Err(Error::EINVAL);
            }

            let Some(buf) = (if let Some(name_len) = memchr::memchr(0, name) {
                buf.try_into_view(
                    std::mem::size_of::<Raw>()..(std::mem::size_of::<Raw>() + name_len),
                )
            } else {
                buf.try_into_view(std::mem::size_of::<Raw>()..)
            }) else {
                return Err(Error::EPROTO);
            };

            Ok(Self {
                parent: ino,
                mode,
                umask,
                rdev,
                buf,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::os::unix::ffi::OsStrExt;

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

        buf.extend_from_slice(&[0u8; std::mem::size_of::<RawHeader>()]);
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
