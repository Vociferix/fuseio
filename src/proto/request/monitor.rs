use super::{Cfg, Ino, RawHeader};
use crate::types::MonitorFlags;
use crate::{Error, Result, buf::Buf};

#[derive(Debug)]
pub struct Monitor {
    ino: Ino,
    flags: MonitorFlags,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct Raw {
    hdr: RawHeader,
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
        let Some((raw, _)) = buf.as_slice().split_first::<Raw>() else {
            return Err(Error::EPROTO);
        };

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        Ok(Self {
            ino,
            flags: MonitorFlags::from_bits_retain(raw.flags),
        })
    }
}
