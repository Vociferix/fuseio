use super::{Cfg, HDR_LEN, Ino};
use crate::types::FileHandle;
use crate::{Buf, Error, Result};

bitflags::bitflags! {
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    pub(super) struct GetAttrFlags: u32 {
        const FH = 1 << 0;
    }
}

#[derive(Debug)]
pub struct GetAttr {
    ino: Ino,
    fh: Option<FileHandle>,
}

#[repr(C)]
struct Raw {
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
            if buf.len() < const { HDR_LEN + std::mem::size_of::<Raw>() } {
                return Err(Error::EPROTO);
            }

            let Some(ino) = ino else {
                return Err(Error::EINVAL);
            };

            let raw = unsafe { &*(buf.as_ptr().add(HDR_LEN) as *const Raw) };

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
