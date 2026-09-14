use super::{Cfg, EncodeNotify, IntoIoBuf, IoBuf, NotifyCode, RawHeader};
use crate::types::Ino;

#[derive(Debug)]
pub struct Retrieve {
    id: u64,
    ino: Ino,
    offset: u64,
    size: usize,
}

impl Retrieve {
    pub fn new(id: u64, ino: Ino, offset: u64, len: usize) -> Self {
        Self {
            id,
            ino,
            offset,
            size: len,
        }
    }
}

impl EncodeNotify for Retrieve {
    type Error = crate::Error;

    fn encode(self, _: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            id: u64,
            ino: u64,
            offset: u64,
            size: u32,
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

        let Ok(size) = u32::try_from(self.size) else {
            return Err(crate::Error::ERANGE);
        };

        Ok(Out {
            hdr: RawHeader {
                len: const { std::mem::size_of::<Out>() as u32 },
                code: NotifyCode::RETRIEVE,
                _zero: 0,
            },
            id: self.id,
            ino: self.ino.as_raw(),
            offset: self.offset,
            size,
            _unused: 0,
        })
    }
}
