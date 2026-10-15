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
