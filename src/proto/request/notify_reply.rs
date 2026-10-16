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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::request::Opcode;
    use crate::proto::request::test_support::{latest, request};

    /// `fuse_notify_retrieve_in { dummy1, offset, size, dummy2, dummy3,
    /// dummy4 }`, then the data the kernel is handing back.
    fn body(offset: u64, size: u32, data: &[u8]) -> Vec<u8> {
        let mut body = 0u64.to_ne_bytes().to_vec(); // dummy1
        body.extend_from_slice(&offset.to_ne_bytes());
        body.extend_from_slice(&size.to_ne_bytes());
        body.extend_from_slice(&0u32.to_ne_bytes()); // dummy2
        body.extend_from_slice(&0u64.to_ne_bytes()); // dummy3
        body.extend_from_slice(&0u64.to_ne_bytes()); // dummy4
        body.extend_from_slice(data);
        body
    }

    fn decode(offset: u64, size: u32, data: &[u8]) -> Result<NotifyReply> {
        NotifyReply::decode(
            request(Opcode::NOTIFY_REPLY, &body(offset, size, data)),
            None,
            latest(),
        )
    }

    #[test]
    fn the_offset_and_data_come_from_the_body() {
        let req = decode(8192, 4, b"data").expect("decodes");

        assert_eq!(req.offset(), 8192);
        assert_eq!(req.data(), b"data");
    }

    // `size` says how much of the reply is the retrieved data; anything after
    // it is padding the kernel added, not content.
    #[test]
    fn the_data_stops_at_the_announced_size() {
        let req = decode(0, 4, b"data\0\0\0\0").expect("decodes");

        assert_eq!(req.data(), b"data");
    }

    // Claiming more data than arrived is a reply that cannot be honoured.
    #[test]
    fn a_size_larger_than_what_arrived_is_refused() {
        assert_eq!(
            decode(0, 64, b"short").map(|_| ()).unwrap_err(),
            Error::EINVAL
        );
    }

    #[test]
    fn a_reply_with_no_data_decodes() {
        let req = decode(0, 0, b"").expect("decodes");

        assert!(req.data().is_empty());
    }

    #[test]
    fn a_truncated_body_is_refused() {
        let err = NotifyReply::decode(request(Opcode::NOTIFY_REPLY, &[0u8; 16]), None, latest())
            .map(|_| ())
            .unwrap_err();

        assert_eq!(err, Error::EPROTO);
    }
}
