use super::{Cfg, HDR_LEN, Ino};
use crate::{Buf, Error, Result};

// NOTE: This is the response to a retrieve notification. It is
//       possible that notification types added in the future
//       will also have a response, in which case this opcode
//       would be used for that also. If that happens, this
//       type will need to support multiple reply types.
#[derive(Debug, Clone)]
pub struct NotifyReply {
    offset: u64,
    buf: Buf,
}

#[repr(C)]
struct Raw {
    _unused0: u64,
    offset: u64,
    size: u32,
    _unused1: u32,
    _unused2: u64,
    _unused3: u64,
}

const DATA_OFFSET: usize = HDR_LEN + std::mem::size_of::<Raw>();

impl NotifyReply {
    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn data(&self) -> &[u8] {
        &self.buf[DATA_OFFSET..]
    }
}

impl NotifyReply {
    pub(super) fn decode(mut buf: Buf, _: Option<Ino>, _: Cfg) -> Result<Self> {
        if buf.len() < DATA_OFFSET {
            return Err(Error::EPROTO);
        }

        let raw = unsafe { std::ptr::read(buf.as_ptr().add(HDR_LEN) as *const Raw) };

        let len = raw.size as usize;

        let Some(expected_len) = len.checked_add(DATA_OFFSET) else {
            return Err(Error::EINVAL);
        };

        if buf.len() < expected_len {
            return Err(Error::EINVAL);
        }

        buf.truncate(expected_len);

        Ok(Self {
            offset: raw.offset,
            buf,
        })
    }
}
