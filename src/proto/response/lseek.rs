use super::{Cfg, EncodeResp, IntoIoBuf, RawHeader};
use crate::buf::Pod;

#[derive(Debug)]
pub struct Lseek {
    offset: u64,
}

impl Lseek {
    pub const fn new(offset: u64) -> Self {
        Self { offset }
    }
}

impl From<u64> for Lseek {
    fn from(offset: u64) -> Self {
        Self::new(offset)
    }
}

impl EncodeResp for Lseek {
    type Error = std::convert::Infallible;

    fn encode(self, id: u64, _: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        #[derive(Clone, Copy, bytemuck::NoUninit)]
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            offset: u64,
        }

        Ok(Pod(Out {
            hdr: RawHeader {
                len: const { std::mem::size_of::<Out>() as u32 },
                err: 0,
                id,
            },
            offset: self.offset,
        }))
    }
}
