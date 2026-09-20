use super::{Cfg, EncodeNotify, IntoIoBuf, IoBuf, NotifyCode, RawHeader};
use crate::buf::Vectored;
use crate::types::Ino;

use compio::buf::IoVectoredBuf;

#[derive(Debug)]
pub struct Store<B> {
    ino: Ino,
    offset: u64,
    data: B,
}

impl<B: IntoIoBuf> Store<B> {
    pub fn new(ino: Ino, offset: u64, data: B) -> Self {
        Self { ino, offset, data }
    }
}

impl<B: IntoIoBuf> EncodeNotify for Store<B> {
    type Error = crate::Error;

    fn encode(self, _: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            nodeid: u64,
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

        let buf = self.data.into_io_buf();
        let size = buf.total_len();

        let Ok(size) = u32::try_from(size) else {
            return Err(crate::Error::ERANGE);
        };

        let Some(len) = const { std::mem::size_of::<Out>() as u32 }.checked_add(size) else {
            return Err(crate::Error::ERANGE);
        };

        Ok(Vectored((
            Out {
                hdr: RawHeader {
                    len,
                    code: NotifyCode::STORE,
                    _zero: 0,
                },
                nodeid: self.ino.as_raw(),
                offset: self.offset,
                size,
                _unused: 0,
            },
            buf,
        )))
    }
}
