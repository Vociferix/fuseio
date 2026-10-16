use super::{Cfg, Ino, RawHeader};
use crate::types::FileHandle;
use crate::{Error, Result, buf::Buf};

bitflags::bitflags! {
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, bytemuck::AnyBitPattern)]
    pub(super) struct GetAttrFlags: u32 {
        const FH = 1 << 0;
    }
}

#[derive(Debug)]
pub struct GetAttr {
    ino: Ino,
    fh: Option<FileHandle>,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct Raw {
    hdr: RawHeader,
    flags: GetAttrFlags,
    _unused: u32,
    fh: u64,
}

impl GetAttr {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> Option<FileHandle> {
        self.fh
    }
}

impl GetAttr {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, cfg: Cfg) -> Result<Self> {
        if cfg.minor_ver >= 9 {
            let Some((raw, _)) = buf.as_slice().split_first::<Raw>() else {
                return Err(Error::EPROTO);
            };

            let Some(ino) = ino else {
                return Err(Error::EINVAL);
            };

            Ok(Self {
                ino,
                fh: raw
                    .flags
                    .contains(GetAttrFlags::FH)
                    .then_some(FileHandle(raw.fh)),
            })
        } else {
            let Some(ino) = ino else {
                return Err(Error::EINVAL);
            };

            Ok(Self { ino, fh: None })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::request::Opcode;
    use crate::proto::request::test_support::{latest, request, root};

    /// `fuse_getattr_in { getattr_flags, dummy, fh }`.
    fn body(flags: u32, fh: u64) -> Vec<u8> {
        let mut body = flags.to_ne_bytes().to_vec();
        body.extend_from_slice(&0xdead_beefu32.to_ne_bytes()); // dummy
        body.extend_from_slice(&fh.to_ne_bytes());
        body
    }

    /// `FUSE_GETATTR_FH`: the handle is only meaningful when this is set.
    const FH_FLAG: u32 = 1 << 0;

    // Without the flag the handle field is not a handle, whatever it holds.
    #[test]
    fn the_handle_is_ignored_unless_the_flag_says_otherwise() {
        let req = GetAttr::decode(request(Opcode::GETATTR, &body(0, 42)), root(), latest())
            .expect("decodes");

        assert_eq!(req.ino(), root().unwrap());
        assert_eq!(req.file_handle(), None);
    }

    #[test]
    fn the_handle_is_used_when_the_flag_is_set() {
        let req = GetAttr::decode(
            request(Opcode::GETATTR, &body(FH_FLAG, 42)),
            root(),
            latest(),
        )
        .expect("decodes");

        assert_eq!(req.file_handle().map(|fh| fh.0), Some(42));
    }

    #[test]
    fn a_truncated_body_is_refused() {
        let err = GetAttr::decode(request(Opcode::GETATTR, &[0u8; 8]), root(), latest())
            .map(|_| ())
            .unwrap_err();

        assert_eq!(err, Error::EPROTO);
    }
}
