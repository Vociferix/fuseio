use super::{Cfg, EncodeNotify, IntoIoBuf, NotifyCode, RawHeader};

use compio::buf::IoBuf;

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
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            kh: u64,
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
                code: NotifyCode::POLL,
                _zero: 0,
            },
            kh: self.kh,
        })
    }
}
