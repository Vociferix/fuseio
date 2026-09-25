use super::{Cfg, HDR_LEN, Ino};
use crate::types::MonitorFlags;
use crate::{Error, Result, buf::Buf};

#[derive(Debug)]
pub struct Monitor {
    ino: Ino,
    flags: MonitorFlags,
}

#[repr(C)]
struct Raw {
    flags: u32,
    _unused: u32,
}

impl Monitor {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn begin(&self) -> bool {
        self.flags.contains(MonitorFlags::BEGIN)
    }

    pub fn end(&self) -> bool {
        self.flags.contains(MonitorFlags::END)
    }
}

impl Monitor {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        if buf.len() < const { std::mem::size_of::<Raw>() + HDR_LEN } {
            return Err(Error::EPROTO);
        }

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let raw = unsafe { &*(buf.as_ptr().add(HDR_LEN) as *const Raw) };

        Ok(Self {
            ino,
            flags: MonitorFlags::from_bits_retain(raw.flags),
        })
    }
}
