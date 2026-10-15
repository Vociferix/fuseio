use super::{Cfg, EncodeNotify, IntoIoBuf, NotifyCode, RawHeader};
use crate::buf::Pod;

#[derive(Debug)]
pub struct Poll {
    kh: u64,
}

impl Poll {
    pub fn new(kh: u64) -> Self {
        Self { kh }
    }
}

impl EncodeNotify for Poll {
    type Error = std::convert::Infallible;

    fn encode(self, _: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        #[derive(Clone, Copy, bytemuck::NoUninit)]
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            kh: u64,
        }

        Ok(Pod(Out {
            hdr: RawHeader {
                len: const { std::mem::size_of::<Out>() as u32 },
                code: NotifyCode::POLL,
                _zero: 0,
            },
            kh: self.kh,
        }))
    }
}
