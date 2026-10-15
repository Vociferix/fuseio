use super::{Cfg, EncodeResp, IntoIoBuf, RawHeader};
use crate::buf::Pod;

use std::time::SystemTime;

#[derive(Debug, Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
pub struct XTimes {
    bkuptime: u64,
    crtime: u64,
    bkuptimensec: u32,
    crtimensec: u32,
}

impl XTimes {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bkuptime(mut self, bkuptime: SystemTime) -> Self {
        let (secs, nanos) = crate::proto::time::split_raw(bkuptime);
        self.bkuptime = secs;
        self.bkuptimensec = nanos;
        self
    }

    pub fn crtime(mut self, crtime: SystemTime) -> Self {
        let (secs, nanos) = crate::proto::time::split_raw(crtime);
        self.crtime = secs;
        self.crtimensec = nanos;
        self
    }
}

impl EncodeResp for XTimes {
    type Error = std::convert::Infallible;

    fn encode(self, id: u64, _: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        #[derive(Clone, Copy, bytemuck::NoUninit)]
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            xtimes: XTimes,
        }

        Ok(Pod(Out {
            hdr: RawHeader {
                len: std::mem::size_of::<Out>() as u32,
                err: 0,
                id,
            },
            xtimes: self,
        }))
    }
}

impl Default for XTimes {
    fn default() -> Self {
        Self {
            bkuptime: 0,
            crtime: u64::MAX,
            bkuptimensec: 0,
            crtimensec: u32::MAX,
        }
    }
}
