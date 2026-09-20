use super::{Cfg, EncodeNotify, IntoIoBuf, IoBuf, NotifyCode, RawHeader};
use crate::buf::Vectored;
use crate::types::Ino;

use compio::buf::IoVectoredBuf;

#[derive(Debug)]
pub struct Entry<B, const EXPIRE: bool> {
    parent: Ino,
    name: B,
}

impl<B: IntoIoBuf, const EXPIRE: bool> Entry<B, EXPIRE> {
    pub fn new(parent: Ino, name: B) -> Self {
        Self { parent, name }
    }
}

impl<B: IntoIoBuf, const EXPIRE: bool> EncodeNotify for Entry<B, EXPIRE> {
    type Error = crate::Error;

    fn encode(self, _: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            parent: u64,
            namelen: u32,
            flags: u32,
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

        let buf = self.name.into_io_buf_with_nul();
        let namelen = buf.total_len();

        let Ok(namelen) = u32::try_from(namelen) else {
            return Err(crate::Error::ERANGE);
        };

        let Some(len) = const { std::mem::size_of::<Out>() as u32 }.checked_add(namelen) else {
            return Err(crate::Error::ERANGE);
        };

        Ok(Vectored((
            Out {
                hdr: RawHeader {
                    len,
                    code: NotifyCode::INVAL_ENTRY,
                    _zero: 0,
                },
                parent: self.parent.as_raw(),
                namelen,
                flags: const { if EXPIRE { 1 } else { 0 } },
            },
            buf,
        )))
    }
}
