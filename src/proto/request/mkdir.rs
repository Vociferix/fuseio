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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::request::Opcode;
    use crate::proto::request::test_support::{latest, request, root};

    /// `fuse_mkdir_in { mode, umask }`, then the name.
    fn body(mode: u32, umask: u32, name: &[u8]) -> Vec<u8> {
        let mut body = mode.to_ne_bytes().to_vec();
        body.extend_from_slice(&umask.to_ne_bytes());
        body.extend_from_slice(name);
        body
    }

    // The mode and the umask are adjacent and both look like modes, so a swap
    // would create a directory with the complement of its permissions.
    // `Mode::bits` is `u32` on Linux and `mode_t` elsewhere.
    #[allow(clippy::unnecessary_cast)]
    #[test]
    fn the_mode_and_umask_are_not_interchangeable() {
        let req = MkDir::decode(
            request(Opcode::MKDIR, &body(0o755, 0o022, b"sub\0")),
            root(),
            latest(),
        )
        .expect("decodes");

        assert_eq!(req.parent(), root().unwrap());
        assert_eq!(req.name(), "sub");
        assert_eq!(req.mode().bits() as u32 & 0o7777, 0o755);
        assert_eq!(req.umask().bits() as u32 & 0o7777, 0o022);
    }

    #[test]
    fn a_truncated_body_is_refused() {
        let err = MkDir::decode(request(Opcode::MKDIR, &[0u8; 4]), root(), latest())
            .map(|_| ())
            .unwrap_err();

        assert_eq!(err, Error::EPROTO);
    }

    #[test]
    fn a_request_without_an_inode_is_refused() {
        let err = MkDir::decode(
            request(Opcode::MKDIR, &body(0o755, 0, b"x\0")),
            None,
            latest(),
        )
        .map(|_| ())
        .unwrap_err();

        assert_eq!(err, Error::EINVAL);
    }
}
