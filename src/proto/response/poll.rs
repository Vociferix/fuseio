use super::{Cfg, EncodeResp, IntoIoBuf, IoBuf, RawHeader};
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
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            ready: u32,
            _unused: u32,
        }

        impl IoBuf for Out {
            fn as_init(&self) -> &[u8] {
                unsafe { std::slice::from_raw_parts(self.buf_ptr(), self.buf_len()) }
            }

            fn buf_ptr(&self) -> *const u8 {
                self as *const Self as *const u8
            }

            fn buf_len(&self) -> usize {
                const { std::mem::size_of::<Out>() }
            }
        }

        Ok(Out {
            hdr: RawHeader {
                len: const { std::mem::size_of::<Out>() as u32 },
                err: 0,
                id,
            },
            ready: self.ready.bits().cast_unsigned().into(),
            _unused: 0,
        })
    }
}
