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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::notify::test_support::{HDR, assert_notify_header, encode, u32_at, u64_at};

    const UNIQUE: usize = HDR;
    const NODEID: usize = HDR + 8;
    const OFFSET: usize = HDR + 16;
    const SIZE: usize = HDR + 24;

    // `fuse_notify_retrieve_out { notify_unique, nodeid, offset, size,
    // padding }`, code 5. The reply comes back as `FUSE_NOTIFY_REPLY` carrying
    // the same unique, which is why it travels in the body rather than the
    // header.
    #[test]
    fn the_request_describes_what_to_send_back() {
        let ino = Ino::from_raw(12).unwrap();
        let bytes = encode(Retrieve::new(77, ino, 8192, 4096));

        assert_notify_header(&bytes, 5);
        assert_eq!(u64_at(&bytes, UNIQUE), 77);
        assert_eq!(u64_at(&bytes, NODEID), 12);
        assert_eq!(u64_at(&bytes, OFFSET), 8192);
        assert_eq!(u32_at(&bytes, SIZE), 4096);
        assert_eq!(u32_at(&bytes, SIZE + 4), 0, "the padding must be zero");
        assert_eq!(bytes.len(), HDR + 32);
    }

    // The notification's own `unique` stays zero: the one in the body is what
    // the reply will quote.
    #[test]
    fn the_header_unique_stays_zero() {
        let ino = Ino::from_raw(1).unwrap();
        let bytes = encode(Retrieve::new(99, ino, 0, 1));

        assert_eq!(u64_at(&bytes, 8), 0);
        assert_eq!(u64_at(&bytes, UNIQUE), 99);
    }
}
