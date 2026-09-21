use super::{Cfg, EncodeResp, RawHeader};
use crate::types::IoctlFlags;
use crate::{
    Result,
    buf::{IntoIoBuf, Vectored},
};

#[derive(Debug)]
pub struct IoctlReply<B> {
    pub(crate) value: i32,
    pub(crate) data: B,
}

#[repr(C)]
struct Raw {
    hdr: RawHeader,
    result: i32,
    flags: IoctlFlags,
    in_iovs: u32,
    out_iovs: u32,
}

impl compio::buf::IoBuf for Raw {
    fn as_init(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.buf_ptr(), self.buf_len()) }
    }

    fn buf_ptr(&self) -> *const u8 {
        self as *const Self as *const u8
    }

    fn buf_len(&self) -> usize {
        std::mem::size_of::<Raw>()
    }
}

impl IoctlReply<[u8; 0]> {
    pub fn empty() -> Self {
        Self { value: 0, data: [] }
    }
}

impl<B> IoctlReply<B> {
    pub fn new(data: B) -> Self {
        Self { value: 0, data }
    }

    pub fn with_raw_return(mut self, value: i32) -> Self {
        self.value = value;
        self
    }

    pub fn with_value(self, value: u32) -> crate::Result<Self> {
        if let Ok(raw) = i32::try_from(value) {
            Ok(self.with_raw_return(raw))
        } else {
            Err(crate::Error::EIO)
        }
    }

    pub fn with_size(self, size: usize) -> crate::Result<Self> {
        if let Ok(raw) = i32::try_from(size) {
            Ok(self.with_raw_return(raw))
        } else {
            Err(crate::Error::EIO)
        }
    }

    pub fn with_error<E>(self, error: E) -> Self
    where
        E: Into<crate::Error>,
    {
        self.with_raw_return(-error.into().raw_os_error())
    }
}

impl<B: IntoIoBuf> EncodeResp for IoctlReply<B> {
    type Error = crate::Error;

    fn encode(self, id: u64, _: Cfg) -> Result<impl IntoIoBuf> {
        let data = self.data.into_io_buf();
        let Some(len) = data.total_len().checked_add(std::mem::size_of::<Raw>()) else {
            return Err(crate::Error::ERANGE);
        };
        let Ok(len) = u32::try_from(len) else {
            return Err(crate::Error::ERANGE);
        };

        let hdr = Raw {
            hdr: RawHeader { len, err: 0, id },
            result: self.value,
            flags: IoctlFlags::empty(),
            in_iovs: 0,
            out_iovs: 0,
        };

        Ok(Vectored((hdr, data.into_vectored())))
    }
}
