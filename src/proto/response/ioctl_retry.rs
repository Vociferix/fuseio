use super::{Cfg, EncodeResp, IoBuf, RawHeader};
use crate::types::IoctlFlags;
use crate::{
    Result,
    buf::{Buf, BufPool, IntoIoBuf, Vectored},
};

use futures_util::{Stream, StreamExt};

use std::pin::pin;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IoctlLookup {
    pub addr: u64,
    pub len: u64,
}

#[derive(Debug)]
pub struct IoctlRetry {
    in_buf: Buf<IoctlLookup>,
    out_buf: Buf<IoctlLookup>,
}

#[repr(C)]
pub(super) struct Raw {
    pub(super) hdr: RawHeader,
    pub(super) result: i32,
    pub(super) flags: IoctlFlags,
    pub(super) in_iovs: u32,
    pub(super) out_iovs: u32,
}

struct IoctlIovecBuf(Buf<IoctlLookup>);

impl IoBuf for Raw {
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

impl IoBuf for IoctlIovecBuf {
    fn as_init(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.buf_ptr(), self.buf_len()) }
    }

    fn buf_ptr(&self) -> *const u8 {
        self.0.as_ptr().cast()
    }

    fn buf_len(&self) -> usize {
        self.0.len() * std::mem::size_of::<IoctlLookup>()
    }
}

impl IoctlRetry {
    pub fn new(pool: &BufPool) -> Self {
        Self {
            in_buf: pool.checkout(),
            out_buf: pool.checkout(),
        }
    }

    pub async fn in_iovs<S>(mut self, iovs: S) -> Result<Self>
    where
        S: Stream<Item = Result<IoctlLookup>>,
    {
        self.in_buf.reserve(iovs.size_hint().0);

        let mut iovs = pin!(iovs);

        while let Some(res) = iovs.next().await {
            self.in_buf.push(res?);
        }

        Ok(self)
    }

    pub async fn out_iovs<S>(mut self, lens: S) -> Result<Self>
    where
        S: Stream<Item = Result<u64>>,
    {
        self.out_buf.reserve(lens.size_hint().0);

        let mut lens = pin!(lens);

        while let Some(res) = lens.next().await {
            self.out_buf.push(IoctlLookup { addr: 0, len: res? });
        }

        Ok(self)
    }
}

impl EncodeResp for IoctlRetry {
    type Error = crate::Error;

    fn encode(self, id: u64, _: Cfg) -> Result<impl IntoIoBuf> {
        let Some(total_iovs) = self.in_buf.len().checked_add(self.out_buf.len()) else {
            return Err(crate::Error::ERANGE);
        };

        if total_iovs > 256 {
            return Err(crate::Error::ERANGE);
        }

        let Some(iovs_len) = total_iovs.checked_mul(std::mem::size_of::<IoctlLookup>()) else {
            return Err(crate::Error::ERANGE);
        };

        let Ok(iovs_len) = u32::try_from(iovs_len) else {
            return Err(crate::Error::ERANGE);
        };

        let Some(len) = iovs_len.checked_add(const { std::mem::size_of::<Raw>() as u32 }) else {
            return Err(crate::Error::ERANGE);
        };

        let hdr = Raw {
            hdr: RawHeader { len, err: 0, id },
            result: 0,
            flags: IoctlFlags::RETRY,
            in_iovs: self.in_buf.len() as u32,
            out_iovs: self.out_buf.len() as u32,
        };

        Ok(Vectored((
            hdr,
            [IoctlIovecBuf(self.in_buf), IoctlIovecBuf(self.out_buf)],
        )))
    }
}
