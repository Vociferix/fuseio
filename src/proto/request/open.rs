use super::{Cfg, HDR_LEN, Ino};
use crate::types::{OFlag, OpenFlags};
use crate::{Error, Result, buf::Buf};

#[derive(Debug)]
pub struct Open {
    ino: Ino,
    flags: OFlag,
    op_flags: OpenFlags,
}

#[repr(C)]
struct Raw {
    flags: u32,
    open_flags: u32,
}

impl Open {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn flags(&self) -> OFlag {
        self.flags
    }

    pub fn op_flags(&self) -> OpenFlags {
        self.op_flags
    }
}

impl Open {
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
            // TODO(e2e): assumes host-native open flag values; verify once end-to-end
            // tests can be done.
            flags: OFlag::from_bits_retain(raw.flags.cast_signed()),
            op_flags: OpenFlags::from_bits_retain(raw.open_flags),
        })
    }
}
