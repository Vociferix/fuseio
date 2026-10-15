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
