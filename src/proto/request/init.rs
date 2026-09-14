use super::HDR_LEN;
use crate::types::{KernelInitFlags, Version};
use crate::{Buf, Error, Result};

#[derive(Debug)]
pub struct Init {
    ver: Version,
    flags: KernelInitFlags,
    max_readahead: usize,
}

#[repr(C)]
struct Raw {
    major: u32,
    minor: u32,
    max_readahead: u32,
    flags: u32,
}

#[repr(C)]
struct RawExt {
    major: u32,
    minor: u32,
    max_readahead: u32,
    flags0: u32,
    flags1: u32,
    _unused: u32,
}

impl Init {
    pub fn version(&self) -> Version {
        self.ver
    }

    pub fn flags(&self) -> KernelInitFlags {
        self.flags
    }

    pub fn max_readahead(&self) -> usize {
        self.max_readahead
    }
}

impl Init {
    pub(super) fn decode(buf: Buf) -> Result<Self> {
        if buf.len() < const { HDR_LEN + std::mem::size_of::<Raw>() } {
            return Err(Error::EPROTO);
        }

        let base = unsafe { &*(buf.as_ptr().add(HDR_LEN) as *const Raw) };
        if base.major < 7 {
            return Err(Error::EPROTO);
        }

        if base.major > 7 {
            return Ok(Self {
                ver: Version(base.major, base.minor),
                flags: KernelInitFlags::from_wire(base.flags, 0),
                max_readahead: base.max_readahead as usize,
            });
        }

        let ver = Version(base.major, base.minor);
        let max_readahead = base.max_readahead as usize;

        let flags2 = if KernelInitFlags::has_flags2(base.flags) {
            if buf.len() < const { HDR_LEN + std::mem::size_of::<RawExt>() } {
                return Err(Error::EPROTO);
            }
            let ext = unsafe { &*(buf.as_ptr().add(HDR_LEN) as *const RawExt) };
            ext.flags1
        } else {
            0
        };
        let flags = KernelInitFlags::from_wire(base.flags, flags2);

        Ok(Self {
            ver,
            flags,
            max_readahead,
        })
    }
}
