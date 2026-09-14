use super::{Cfg, EncodeNotify, IntoIoBuf, NotifyCode, RawHeader};
use crate::types::Ino;

use compio::buf::IoBuf;

use std::ops::{Bound, RangeBounds};

#[repr(C)]
#[derive(Debug)]
pub struct InvalInode {
    ino: u64,
    offset: u64,
    len: u64,
}

impl InvalInode {
    pub fn new(ino: Ino) -> Self {
        Self {
            ino: ino.as_raw(),
            offset: 0,
            len: 0,
        }
    }

    pub fn range<R>(mut self, range: R) -> Self
    where
        R: RangeBounds<u64>,
    {
        let start = match range.start_bound() {
            Bound::Unbounded => 0,
            Bound::Included(start) => *start,
            Bound::Excluded(start) => *start + 1,
        };

        let len = match range.end_bound() {
            Bound::Unbounded => 0,
            Bound::Included(end) => *end + 1 - start,
            Bound::Excluded(end) => *end - start,
        };

        self.offset = start;
        self.len = len;

        self
    }
}

impl EncodeNotify for InvalInode {
    type Error = std::convert::Infallible;

    fn encode(self, _: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            notify: InvalInode,
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
                code: NotifyCode::INVAL_INODE,
                _zero: 0,
            },
            notify: self,
        })
    }
}
