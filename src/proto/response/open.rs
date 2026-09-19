use super::{Cfg, EncodeResp, IntoIoBuf, IoBuf, RawHeader};
use crate::fs::types::PassthroughFd;
use crate::types::{FileHandle, OpenedFlags};

use std::os::fd::AsFd;

#[repr(C)]
#[derive(Debug)]
pub struct Opened {
    fh: u64,
    open_flags: u32,
    backing_id: u32,
}

const PASSTHROUGH: u32 = 1u32 << 7;

impl Opened {
    pub fn new(fh: FileHandle) -> Self {
        Self {
            fh: fh.0,
            open_flags: 0,
            backing_id: 0,
        }
    }

    pub fn flags(mut self, flags: OpenedFlags) -> Self {
        self.open_flags = flags.bits();
        self
    }

    pub fn add_flags(mut self, flags: OpenedFlags) -> Self {
        self.open_flags |= flags.bits();
        self
    }

    pub fn passthrough<T: AsFd>(mut self, fd: &PassthroughFd<T>) -> Self {
        self.open_flags |= PASSTHROUGH;
        self.backing_id = PassthroughFd::backing_id(fd).0;
        self
    }
}

impl EncodeResp for Opened {
    type Error = std::convert::Infallible;

    fn encode(self, id: u64, _: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            open: Opened,
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
            open: self,
        })
    }
}
