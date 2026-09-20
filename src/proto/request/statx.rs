use super::getattr::GetAttrFlags;
use super::{Cfg, HDR_LEN, Ino};
use crate::types::{FileHandle, StatXMask, StatXSync};
use crate::{Error, Result, buf::Buf};

#[derive(Debug)]
pub struct StatX {
    ino: Ino,
    fh: Option<FileHandle>,
    sync: StatXSync,
    mask: StatXMask,
}

#[repr(C)]
struct Raw {
    getattr_flags: GetAttrFlags,
    _unused: u32,
    fh: u64,
    flags: u32,
    mask: StatXMask,
}

impl StatX {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> Option<FileHandle> {
        self.fh
    }

    pub fn sync_mode(&self) -> StatXSync {
        self.sync
    }

    pub fn field_mask(&self) -> StatXMask {
        self.mask
    }
}

impl StatX {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        if buf.len() < const { HDR_LEN + std::mem::size_of::<Raw>() } {
            return Err(Error::EPROTO);
        }

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let raw = unsafe { &*(buf.as_ptr().add(HDR_LEN) as *const Raw) };

        // Only Linux sends STATX, so Linux's values are the host-native ones.
        // Literals because libc omits them on older musl targets.
        const AT_STATX_SYNC_TYPE: i32 = 0x6000;
        const AT_STATX_SYNC_AS_STAT: i32 = 0x0000;
        const AT_STATX_FORCE_SYNC: i32 = 0x2000;
        const AT_STATX_DONT_SYNC: i32 = 0x4000;

        let sync = match raw.flags.cast_signed() & AT_STATX_SYNC_TYPE {
            AT_STATX_SYNC_AS_STAT => StatXSync::AsStat,
            AT_STATX_DONT_SYNC => StatXSync::DontSync,
            AT_STATX_FORCE_SYNC => StatXSync::Force,
            _ => return Err(Error::EINVAL),
        };

        Ok(Self {
            ino,
            fh: raw
                .getattr_flags
                .contains(GetAttrFlags::FH)
                .then_some(FileHandle(raw.fh)),
            sync,
            mask: raw.mask,
        })
    }
}
