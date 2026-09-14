use super::{Cfg, HDR_LEN, Ino};
use crate::types::AccessFlags;
use crate::{Buf, Error, Result};

#[derive(Debug)]
pub struct Access {
    ino: Ino,
    flags: AccessFlags,
}

#[repr(C)]
struct Raw {
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
        if buf.len() < const { HDR_LEN + std::mem::size_of::<Raw>() } {
            return Err(Error::EPROTO);
        }

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let raw = unsafe { &*(buf.as_ptr().add(HDR_LEN) as *const Raw) };

        Ok(Self {
            ino,
            // TODO(e2e): assumes host-native access(2) mask values; verify once
            // end-to-end tests can be done.
            flags: AccessFlags::from_bits_retain(raw.mask.cast_signed()),
        })
    }
}
