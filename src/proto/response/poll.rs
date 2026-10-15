use super::{Cfg, EncodeResp, IntoIoBuf, RawHeader};
use crate::buf::Pod;
use crate::types::PollFlags;

#[derive(Debug)]
pub struct Poll {
    ready: PollFlags,
}

impl Poll {
    pub const fn new(ready: PollFlags) -> Self {
        Self { ready }
    }
}

impl From<PollFlags> for Poll {
    fn from(ready: PollFlags) -> Self {
        Self::new(ready)
    }
}

impl EncodeResp for Poll {
    type Error = std::convert::Infallible;

    fn encode(self, id: u64, _: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        #[derive(Clone, Copy, bytemuck::NoUninit)]
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            ready: u32,
            _unused: u32,
        }

        Ok(Pod(Out {
            hdr: RawHeader {
                len: const { std::mem::size_of::<Out>() as u32 },
                err: 0,
                id,
            },
            ready: self.ready.bits().cast_unsigned().into(),
            _unused: 0,
        }))
    }
}
