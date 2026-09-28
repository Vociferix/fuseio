use super::{Cfg, EncodeNotify, IntoIoBuf, NotifyCode, RawHeader};
use crate::types::{FileRange, Ino};

use compio::buf::IoBuf;

/// An offset no data reaches, which every kernel reads as "the attributes only".
const ATTRS_ONLY: i64 = -1;

/// A length every kernel reads as "to the end of the file".
const TO_THE_END: i64 = 0;

#[repr(C)]
#[derive(Debug)]
pub struct InvalInode {
    ino: u64,
    offset: i64,
    len: i64,
}

impl InvalInode {
    /// Invalidates the cached attributes and every cached byte.
    pub fn new(ino: Ino) -> Self {
        Self {
            ino: ino.as_raw(),
            offset: 0,
            len: TO_THE_END,
        }
    }

    /// Invalidates the cached attributes, leaving cached data alone.
    pub fn attrs(ino: Ino) -> Self {
        Self {
            ino: ino.as_raw(),
            offset: ATTRS_ONLY,
            len: TO_THE_END,
        }
    }

    pub fn range<R>(mut self, range: R) -> Self
    where
        R: Into<FileRange>,
    {
        let range = range.into();

        // A length that doesn't fit the signed field, like one reaching the
        // largest offset, is sent as "to the end of the file", which is what it
        // means anyway.
        self.offset = i64::try_from(range.start_offset()).unwrap_or(i64::MAX);
        self.len = range
            .len()
            .and_then(|len| i64::try_from(len).ok())
            .unwrap_or(TO_THE_END);

        self
    }
}

impl EncodeNotify for InvalInode {
    type Error = std::convert::Infallible;

    fn encode(self, _: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            notify: InvalInode,
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

        Ok(Out {
            hdr: RawHeader {
                len: const { std::mem::size_of::<Out>() as u32 },
                code: NotifyCode::INVAL_INODE,
                _zero: 0,
            },
            notify: self,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use compio::buf::IoVectoredBuf;

    const OFFSET: usize = 16 + 8;
    const LEN: usize = 16 + 16;

    fn cfg() -> Cfg {
        Cfg {
            minor_ver: crate::handshake::MINOR_VER,
            ..Cfg::default()
        }
    }

    fn encode(notify: InvalInode) -> (i64, i64) {
        let buf = notify.encode(cfg()).unwrap().into_io_buf();
        let bytes: Vec<u8> = buf.iter_slice().flatten().copied().collect();

        assert_eq!(bytes.len(), 16 + 24);

        (
            i64::from_ne_bytes(bytes[OFFSET..OFFSET + 8].try_into().unwrap()),
            i64::from_ne_bytes(bytes[LEN..LEN + 8].try_into().unwrap()),
        )
    }

    fn ino() -> Ino {
        Ino::from_raw(3).unwrap()
    }

    #[test]
    fn everything_is_invalidated_from_the_start() {
        assert_eq!(encode(InvalInode::new(ino())), (0, TO_THE_END));
    }

    // Every kernel reads a negative offset as "don't touch cached data".
    #[test]
    fn only_the_attributes_can_be_invalidated() {
        let (offset, _) = encode(InvalInode::attrs(ino()));

        assert!(offset < 0);
    }

    #[test]
    fn a_range_becomes_an_offset_and_a_length() {
        assert_eq!(
            encode(InvalInode::new(ino()).range(4096..8192)),
            (4096, 4096)
        );
        assert_eq!(
            encode(InvalInode::new(ino()).range(4096..=8191)),
            (4096, 4096)
        );
        assert_eq!(
            encode(InvalInode::new(ino()).range(4096..)),
            (4096, TO_THE_END)
        );
    }

    #[test]
    fn a_range_reaching_the_largest_offset_doesnt_overflow() {
        assert_eq!(
            encode(InvalInode::new(ino()).range(0..=u64::MAX)),
            (0, TO_THE_END)
        );
        assert_eq!(
            encode(InvalInode::new(ino()).range(4096..=u64::MAX)),
            (4096, TO_THE_END)
        );
    }

    #[test]
    fn an_absurd_length_becomes_to_the_end() {
        // Longer than a signed offset can express.
        let (offset, len) = encode(InvalInode::new(ino()).range(0..u64::MAX));

        assert_eq!(offset, 0);
        assert_eq!(len, TO_THE_END);
    }
}
