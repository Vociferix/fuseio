use super::{Cfg, EncodeResp, IntoIoBuf, RawHeader};
use crate::buf::Pod;

#[derive(Debug)]
pub struct CopyFileRange {
    size: u64,
}

impl CopyFileRange {
    pub const fn new(size: u64) -> Self {
        Self { size }
    }
}

impl From<u64> for CopyFileRange {
    fn from(size: u64) -> Self {
        Self::new(size)
    }
}

impl EncodeResp for CopyFileRange {
    type Error = std::convert::Infallible;

    fn encode(self, id: u64, _: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        #[derive(Clone, Copy, bytemuck::NoUninit)]
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            size: u64,
        }

        Ok(Pod(Out {
            hdr: RawHeader {
                len: const { std::mem::size_of::<Out>() as u32 },
                err: 0,
                id,
            },
            size: self.size,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::response::Write;

    use compio::buf::IoVectoredBuf;

    fn cfg() -> Cfg {
        Cfg {
            minor_ver: crate::handshake::MINOR_VER,
            ..Cfg::default()
        }
    }

    fn bytes<T: EncodeResp>(resp: T) -> Vec<u8>
    where
        crate::Error: From<T::Error>,
    {
        let buf = resp.encode(7, cfg()).ok().unwrap().into_io_buf();

        buf.iter_slice().flatten().copied().collect()
    }

    #[test]
    fn the_64_bit_reply_carries_a_u64() {
        let encoded = bytes(CopyFileRange::new(1 << 33));

        assert_eq!(encoded.len(), 24);
        assert_eq!(u32::from_ne_bytes(encoded[0..4].try_into().unwrap()), 24);
        assert_eq!(
            u64::from_ne_bytes(encoded[16..24].try_into().unwrap()),
            1 << 33
        );
    }

    // A `fuse_write_out` and the 64-bit reply have the same length, and on a
    // little-endian host the same bytes for a small count, so this pins the
    // padding rather than telling the two apart.
    #[test]
    fn the_32_bit_reply_is_a_write_out() {
        let encoded = bytes(Write::new(4096));

        assert_eq!(encoded.len(), 24);
        assert_eq!(u32::from_ne_bytes(encoded[0..4].try_into().unwrap()), 24);
        assert_eq!(
            u32::from_ne_bytes(encoded[16..20].try_into().unwrap()),
            4096
        );
        assert_eq!(u32::from_ne_bytes(encoded[20..24].try_into().unwrap()), 0);
    }
}
