use super::{Cfg, EncodeResp, IntoIoBuf, IoBuf, RawHeader};

#[derive(Debug)]
pub struct Write {
    size: usize,
}

impl Write {
    pub const fn new(size: usize) -> Self {
        Self { size }
    }
}

impl From<usize> for Write {
    fn from(size: usize) -> Self {
        Self::new(size)
    }
}

impl EncodeResp for Write {
    type Error = crate::Error;

    fn encode(self, id: u64, _: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            size: u32,
            _unused: u32,
        }

        impl IoBuf for Out {
            fn as_init(&self) -> &[u8] {
                unsafe { std::slice::from_raw_parts(self.buf_ptr(), self.buf_len()) }
            }

            fn buf_ptr(&self) -> *const u8 {
                self as *const Self as *const u8
            }

            fn buf_len(&self) -> usize {
                const { std::mem::size_of::<Out>() }
            }
        }

        let Ok(size) = u32::try_from(self.size) else {
            log::error!("bytes written ({}) does not fit in a reply", self.size);
            return Err(crate::Error::EIO);
        };

        Ok(Out {
            hdr: RawHeader {
                len: const { std::mem::size_of::<Out>() as u32 },
                err: 0,
                id,
            },
            size,
            _unused: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::types::ReplyInitFlags;

    use compio::buf::IoVectoredBuf;

    fn cfg() -> Cfg {
        Cfg {
            minor_ver: crate::handshake::MINOR_VER,
            ..Cfg::default()
        }
    }

    #[test]
    fn the_count_reaches_the_wire() {
        let buf = Write::new(4096).encode(7, cfg()).unwrap().into_io_buf();
        let bytes: Vec<u8> = buf.iter_slice().flatten().copied().collect();

        assert_eq!(bytes.len(), 24);
        assert_eq!(u32::from_ne_bytes(bytes[16..20].try_into().unwrap()), 4096);
    }

    // Truncating would tell the kernel a different number of bytes was written.
    #[test]
    fn a_count_too_large_to_report_is_refused() {
        let err = Write::new(u32::MAX as usize + 1)
            .encode(7, cfg())
            .map(|_| ())
            .unwrap_err();

        assert_eq!(err, crate::Error::EIO);
    }
}
