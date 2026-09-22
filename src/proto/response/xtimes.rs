use super::{Cfg, EncodeResp, IntoIoBuf, IoBuf, RawHeader};

use std::time::{Duration, SystemTime};

#[derive(Debug)]
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
        let ts = bkuptime
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO);
        self.bkuptime = ts.as_secs();
        self.bkuptimensec = ts.subsec_nanos();
        self
    }

    pub fn crtime(mut self, crtime: SystemTime) -> Self {
        let ts = crtime
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO);
        self.crtime = ts.as_secs();
        self.crtimensec = ts.subsec_nanos();
        self
    }
}

impl EncodeResp for XTimes {
    type Error = std::convert::Infallible;

    fn encode(self, id: u64, _: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            xtimes: XTimes,
        }

        impl IoBuf for Out {
            fn as_init(&self) -> &[u8] {
                unsafe { std::slice::from_raw_parts(self.buf_ptr(), self.buf_len()) }
            }

            fn buf_ptr(&self) -> *const u8 {
                self as *const Self as *const u8
            }

            fn buf_len(&self) -> usize {
                std::mem::size_of::<Out>()
            }
        }

        Ok(Out {
            hdr: RawHeader {
                len: std::mem::size_of::<Out>() as u32,
                err: 0,
                id,
            },
            xtimes: self,
        })
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
