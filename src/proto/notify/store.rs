use super::{Cfg, EncodeNotify, IntoIoBuf, NotifyCode, RawHeader};
use crate::buf::{Pod, Vectored};
use crate::types::Ino;

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
        #[derive(Clone, Copy, bytemuck::NoUninit)]
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            nodeid: u64,
            offset: u64,
            size: u32,
            _unused: u32,
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
            Pod(Out {
                hdr: RawHeader {
                    len,
                    code: NotifyCode::STORE,
                    _zero: 0,
                },
                nodeid: self.ino.as_raw(),
                offset: self.offset,
                size,
                _unused: 0,
            }),
            buf,
        )))
    }
}
