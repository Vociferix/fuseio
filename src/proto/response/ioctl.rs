use super::ioctl_retry::Raw;
use super::{Cfg, EncodeResp, RawHeader};
use crate::types::IoctlFlags;
use crate::{
    Result,
    buf::{IntoIoBuf, Vectored},
};

#[derive(Debug)]
pub struct Ioctl<B> {
    result: i32,
    data: B,
}

impl Ioctl<[u8; 0]> {
    pub fn new() -> Self {
        Self {
            result: 0,
            data: [],
        }
    }
}

impl<B> Ioctl<B> {
    pub fn result(mut self, result: i32) -> Self {
        self.result = result;
        self
    }

    pub fn error(mut self, error: crate::Error) -> Self {
        // TODO(e2e): assumes host-native errno values; verify once end-to-end
        // tests can be done.
        self.result = error.raw_os_error();
        self
    }

    pub fn data<T>(self, data: T) -> Ioctl<T>
    where
        T: IntoIoBuf,
    {
        Ioctl {
            result: self.result,
            data,
        }
    }
}

impl<B: IntoIoBuf> EncodeResp for Ioctl<B> {
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
            result: self.result,
            flags: IoctlFlags::empty(),
            in_iovs: 0,
            out_iovs: 0,
        };

        Ok(Vectored((hdr, data.into_vectored())))
    }
}
