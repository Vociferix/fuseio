use super::{Cfg, Ino, RawHeader};
use crate::types::OFlag;
use crate::{Error, Result, buf::Buf};

/// `FUSE_OPEN_KILL_SUIDGID`.
pub(super) const KILL_SUIDGID: u32 = 1 << 0;

#[derive(Debug)]
pub struct Open {
    ino: Ino,
    flags: OFlag,
    remove_suid_sgid: bool,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct Raw {
    hdr: RawHeader,
    flags: u32,
    open_flags: u32,
}

impl Open {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    /// The flags the file was opened with.
    pub fn open_flags(&self) -> OFlag {
        self.flags
    }

    /// Whether to clear the setuid bit, and the setgid bit if the file is
    /// group-executable.
    pub fn remove_suid_sgid(&self) -> bool {
        self.remove_suid_sgid
    }
}

impl Open {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some((raw, _)) = buf.as_slice().split_first::<Raw>() else {
            return Err(Error::EPROTO);
        };

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        Ok(Self {
            ino,
            // TODO(e2e): assumes host-native open flag values; verify once end-to-end
            // tests can be done.
            flags: OFlag::from_bits_retain(raw.flags.cast_signed()),
            remove_suid_sgid: raw.open_flags & KILL_SUIDGID != 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::request::Opcode;
    use crate::proto::request::test_support::{latest, request, root};

    /// `fuse_open_in { flags, open_flags }`.
    fn body(flags: u32, open_flags: u32) -> Vec<u8> {
        let mut body = flags.to_ne_bytes().to_vec();
        body.extend_from_slice(&open_flags.to_ne_bytes());
        body
    }

    /// `FUSE_OPEN_KILL_SUIDGID`.
    const KILL_SUIDGID: u32 = 1 << 0;

    #[test]
    fn the_open_flags_come_from_the_first_field() {
        let wanted = (nix::libc::O_RDWR | nix::libc::O_APPEND) as u32;
        let req = Open::decode(request(Opcode::OPEN, &body(wanted, 0)), root(), latest())
            .expect("decodes");

        assert_eq!(req.ino(), root().unwrap());
        assert!(req.open_flags().contains(nix::fcntl::OFlag::O_APPEND));
        assert!(!req.remove_suid_sgid());
    }

    // The two fields are adjacent `u32`s, so reading one for the other would
    // turn an ordinary open into one that strips setuid bits.
    #[test]
    fn the_second_field_asks_to_strip_setuid() {
        let req = Open::decode(
            request(Opcode::OPEN, &body(0, KILL_SUIDGID)),
            root(),
            latest(),
        )
        .expect("decodes");

        assert!(req.remove_suid_sgid());
        assert!(req.open_flags().bits() as u32 == 0);
    }

    #[test]
    fn a_truncated_body_is_refused() {
        let err = Open::decode(request(Opcode::OPEN, &[0u8; 4]), root(), latest())
            .map(|_| ())
            .unwrap_err();

        assert_eq!(err, Error::EPROTO);
    }
}
