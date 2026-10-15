use super::{Cfg, Ino, RawHeader};
use crate::buf::{Buf, BufPool};
use crate::{Error, Result};

use aligned_vec::{AVec, ConstAlign};

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

#[derive(Debug, Clone)]
pub struct SharedNotifyReply {
    offset: u64,
    buf: AVec<u8, ConstAlign<{ crate::buf::ALIGN }>>,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct Raw {
    hdr: RawHeader,
    _unused0: u64,
    offset: u64,
    size: u32,
    _unused1: u32,
    _unused2: u64,
    _unused3: u64,
}

impl NotifyReply {
    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn data(&self) -> &[u8] {
        &self.buf[std::mem::size_of::<Raw>()..]
    }

    pub(crate) fn share(self) -> SharedNotifyReply {
        SharedNotifyReply {
            offset: self.offset,
            buf: self.buf.steal(),
        }
    }
}

impl NotifyReply {
    pub(super) fn decode(mut buf: Buf, _: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some((raw, data)) = buf.as_slice().split_first::<Raw>() else {
            return Err(Error::EPROTO);
        };

        let len = raw.size as usize;

        if data.len() < len {
            return Err(Error::EINVAL);
        }

        let offset = raw.offset;

        buf.truncate(std::mem::size_of::<Raw>() + len);

        Ok(Self { offset, buf })
    }
}

impl SharedNotifyReply {
    pub fn bind(self, pool: &BufPool) -> NotifyReply {
        NotifyReply {
            offset: self.offset,
            buf: Buf::from_avec(self.buf, pool.clone()),
        }
    }
}
