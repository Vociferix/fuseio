use super::{Cfg, EncodeResp, IntoIoBuf, IoBuf, RawHeader};

#[derive(Debug)]
pub struct Bmap {
    block: u64,
}

impl Bmap {
    pub fn new(block: u64) -> Self {
        Self { block }
    }
}

impl EncodeResp for Bmap {
    type Error = std::convert::Infallible;

    fn encode(self, id: u64, _: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            block: u64,
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
            block: self.block,
        })
    }
}
