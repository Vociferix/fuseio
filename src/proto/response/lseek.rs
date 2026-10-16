use super::{Cfg, EncodeResp, IntoIoBuf, RawHeader};
use crate::buf::Pod;

#[derive(Debug)]
pub struct Lseek {
    offset: u64,
}

impl Lseek {
    pub const fn new(offset: u64) -> Self {
        Self { offset }
    }
}

impl From<u64> for Lseek {
    fn from(offset: u64) -> Self {
        Self::new(offset)
    }
}

impl EncodeResp for Lseek {
    type Error = std::convert::Infallible;

    fn encode(self, id: u64, _: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        #[derive(Clone, Copy, bytemuck::NoUninit)]
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            offset: u64,
        }

        Ok(Pod(Out {
            hdr: RawHeader {
                len: const { std::mem::size_of::<Out>() as u32 },
                err: 0,
                id,
            },
            offset: self.offset,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::response::test_support::{HDR, assert_reply_header, encode, u64_at};

    // `fuse_lseek_out { offset }`.
    #[test]
    fn the_offset_follows_the_header() {
        let bytes = encode(Lseek::new(4096), 11);

        assert_reply_header(&bytes, 11);
        assert_eq!(bytes.len(), HDR + 8);
        assert_eq!(u64_at(&bytes, HDR), 4096);
    }

    // `SEEK_DATA` past the last hole legitimately answers with the file size,
    // and a zero offset is just as legitimate an answer.
    #[test]
    fn a_zero_offset_is_still_an_answer() {
        let bytes = encode(Lseek::new(0), 1);

        assert_eq!(u64_at(&bytes, HDR), 0);
        assert_reply_header(&bytes, 1);
    }
}
