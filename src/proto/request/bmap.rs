use super::{Cfg, Ino, RawHeader};
use crate::{Error, Result, buf::Buf};

#[derive(Debug)]
pub struct Bmap {
    ino: Ino,
    block: u64,
    block_size: usize,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct Raw {
    hdr: RawHeader,
    block: u64,
    block_size: u32,
    _unused: u32,
}

impl Bmap {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn block(&self) -> u64 {
        self.block
    }

    pub fn block_size(&self) -> usize {
        self.block_size
    }
}

impl Bmap {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some((raw, _)) = buf.as_slice().split_first::<Raw>() else {
            return Err(Error::EPROTO);
        };

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        Ok(Self {
            ino,
            block: raw.block,
            block_size: raw.block_size as usize,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::request::Opcode;
    use crate::proto::request::test_support::{latest, request, root};

    /// `fuse_bmap_in { block, blocksize, padding }`.
    fn body(block: u64, block_size: u32) -> Vec<u8> {
        let mut body = block.to_ne_bytes().to_vec();
        body.extend_from_slice(&block_size.to_ne_bytes());
        body.extend_from_slice(&0u32.to_ne_bytes());
        body
    }

    #[test]
    fn the_block_and_its_size_come_from_the_body() {
        let req = Bmap::decode(request(Opcode::BMAP, &body(17, 4096)), root(), latest())
            .expect("decodes");

        assert_eq!(req.ino(), root().unwrap());
        assert_eq!(req.block(), 17);
        assert_eq!(req.block_size(), 4096);
    }

    #[test]
    fn a_truncated_body_is_refused() {
        let err = Bmap::decode(request(Opcode::BMAP, &[0u8; 8]), root(), latest())
            .map(|_| ())
            .unwrap_err();

        assert_eq!(err, Error::EPROTO);
    }

    #[test]
    fn a_request_without_an_inode_is_refused() {
        let err = Bmap::decode(request(Opcode::BMAP, &body(1, 512)), None, latest())
            .map(|_| ())
            .unwrap_err();

        assert_eq!(err, Error::EINVAL);
    }
}
