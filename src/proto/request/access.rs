use super::{Cfg, Ino, RawHeader};
use crate::buf::Buf;
use crate::types::AccessFlags;
use crate::{Error, Result};

#[derive(Debug)]
pub struct Access {
    ino: Ino,
    flags: AccessFlags,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct Raw {
    hdr: RawHeader,
    mask: u32,
    _unused: u32,
}

impl Access {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn flags(&self) -> AccessFlags {
        self.flags
    }
}

impl Access {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some((raw, _)) = buf.as_slice().split_first::<Raw>() else {
            return Err(Error::EPROTO);
        };

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        Ok(Self {
            ino,
            // TODO(e2e): assumes host-native access(2) mask values; verify once
            // end-to-end tests can be done.
            flags: AccessFlags::from_bits_retain(raw.mask.cast_signed()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::request::Opcode;
    use crate::proto::request::test_support::{latest, request, root};

    /// `fuse_access_in { mask, padding }`. The padding is given a value of its
    /// own so that reading the mask from the wrong half shows up.
    fn body(mask: u32) -> Vec<u8> {
        let mut body = mask.to_ne_bytes().to_vec();
        body.extend_from_slice(&0xdead_beefu32.to_ne_bytes());
        body
    }

    #[test]
    fn the_mask_comes_from_the_body() {
        let mask = (nix::libc::R_OK | nix::libc::X_OK) as u32;
        let req = Access::decode(request(Opcode::ACCESS, &body(mask)), root(), latest())
            .expect("decodes");

        assert_eq!(req.ino(), root().unwrap());
        assert_eq!(req.flags().bits() as u32, mask);
    }

    // `F_OK` is zero: the caller is asking whether the file exists at all.
    #[test]
    fn an_empty_mask_asks_only_whether_it_exists() {
        let req =
            Access::decode(request(Opcode::ACCESS, &body(0)), root(), latest()).expect("decodes");

        assert!(req.flags().is_empty());
    }

    #[test]
    fn a_truncated_body_is_refused() {
        let err = Access::decode(request(Opcode::ACCESS, &[0u8; 2]), root(), latest())
            .map(|_| ())
            .unwrap_err();

        assert_eq!(err, Error::EPROTO);
    }

    #[test]
    fn a_request_without_an_inode_is_refused() {
        let err = Access::decode(request(Opcode::ACCESS, &body(4)), None, latest())
            .map(|_| ())
            .unwrap_err();

        assert_eq!(err, Error::EINVAL);
    }
}
