use super::{Cfg, EncodeNotify, IntoIoBuf, NotifyCode, RawHeader};
use crate::buf::Pod;
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
        #[derive(Clone, Copy, bytemuck::NoUninit)]
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            id: u64,
            ino: u64,
            offset: u64,
            size: u32,
            _unused: u32,
        }

        let Ok(size) = u32::try_from(self.size) else {
            return Err(crate::Error::ERANGE);
        };

        Ok(Pod(Out {
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
        }))
    }
}
