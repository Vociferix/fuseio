use super::{Cfg, EncodeResp, IntoIoBuf, RawHeader};
use crate::buf::Pod;

#[derive(Debug)]
pub struct Bmap {
    block: u64,
}

impl Bmap {
    pub fn new(block: u64) -> Self {
        Self { block }
    }
}

impl EncodeResp for Bmap {
    type Error = std::convert::Infallible;

    fn encode(self, id: u64, _: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        #[derive(Clone, Copy, bytemuck::NoUninit)]
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            block: u64,
        }

        Ok(Pod(Out {
            hdr: RawHeader {
                len: const { std::mem::size_of::<Out>() as u32 },
                err: 0,
                id,
            },
            block: self.block,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::response::test_support::{HDR, assert_reply_header, encode, u64_at};

    // `fuse_bmap_out { block }`.
    #[test]
    fn the_block_number_follows_the_header() {
        let bytes = encode(Bmap::new(0xdead_beef_1234), 7);

        assert_reply_header(&bytes, 7);
        assert_eq!(bytes.len(), HDR + 8);
        assert_eq!(u64_at(&bytes, HDR), 0xdead_beef_1234);
    }
}
