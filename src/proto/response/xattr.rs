use super::{Cfg, EncodeResp, IntoIoBuf, RawHeader};
use crate::buf::Pod;

#[derive(Debug)]
pub struct XattrLen {
    size: u32,
}

impl XattrLen {
    pub const fn new(size: u32) -> Self {
        Self { size }
    }
}

impl EncodeResp for XattrLen {
    type Error = std::convert::Infallible;

    fn encode(self, id: u64, _: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        #[derive(Clone, Copy, bytemuck::NoUninit)]
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            size: u32,
            _unused: u32,
        }

        Ok(Pod(Out {
            hdr: RawHeader {
                len: const { std::mem::size_of::<Out>() as u32 },
                err: 0,
                id,
            },
            size: self.size,
            _unused: 0,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use compio::buf::IoVectoredBuf;

    fn cfg() -> Cfg {
        Cfg {
            minor_ver: crate::handshake::MINOR_VER,
            ..Cfg::default()
        }
    }

    #[test]
    fn the_size_reaches_the_wire() {
        let buf = XattrLen::new(300).encode(7, cfg()).unwrap().into_io_buf();
        let bytes: Vec<u8> = buf.iter_slice().flatten().copied().collect();

        assert_eq!(bytes.len(), 24);
        assert_eq!(u32::from_ne_bytes(bytes[16..20].try_into().unwrap()), 300);
    }
}
