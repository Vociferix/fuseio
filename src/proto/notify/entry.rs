use super::{Cfg, EncodeNotify, IntoIoBuf, IoBuf, NotifyCode, RawHeader};
use crate::buf::Vectored;
use crate::types::Ino;

use compio::buf::IoVectoredBuf;

#[derive(Debug)]
pub struct Entry<B, const EXPIRE: bool> {
    parent: Ino,
    name: B,
}

impl<B: IntoIoBuf, const EXPIRE: bool> Entry<B, EXPIRE> {
    pub fn new(parent: Ino, name: B) -> Self {
        Self { parent, name }
    }
}

impl<B: IntoIoBuf, const EXPIRE: bool> EncodeNotify for Entry<B, EXPIRE> {
    type Error = crate::Error;

    fn encode(self, _: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            parent: u64,
            namelen: u32,
            flags: u32,
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

        let buf = self.name.into_io_buf_with_nul();

        // The kernel counts the name without its terminator, but expects the
        // terminator in the payload.
        let Ok(namelen) = u32::try_from(buf.len_without_nul()) else {
            return Err(crate::Error::ERANGE);
        };

        let Some(len) = u32::try_from(buf.total_len())
            .ok()
            .and_then(|payload| const { std::mem::size_of::<Out>() as u32 }.checked_add(payload))
        else {
            return Err(crate::Error::ERANGE);
        };

        Ok(Vectored((
            Out {
                hdr: RawHeader {
                    len,
                    code: NotifyCode::INVAL_ENTRY,
                    _zero: 0,
                },
                parent: self.parent.as_raw(),
                namelen,
                flags: const { if EXPIRE { 1 } else { 0 } },
            },
            buf,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NAME: &[u8] = b"a-name";

    // `fuse_notify_inval_entry_out` behind a 16-byte header.
    const OUT_LEN: usize = 32;
    const NAMELEN_OFFSET: usize = 24;
    const FLAGS_OFFSET: usize = 28;

    fn cfg() -> Cfg {
        Cfg {
            minor_ver: crate::handshake::MINOR_VER,
            ..Cfg::default()
        }
    }

    fn encode<const EXPIRE: bool>(name: &'static [u8]) -> Vec<u8> {
        let parent = Ino::from_raw(3).unwrap();
        let notify = Entry::<_, EXPIRE>::new(parent, name).encode(cfg()).unwrap();

        notify
            .into_io_buf()
            .iter_slice()
            .flatten()
            .copied()
            .collect()
    }

    fn field(bytes: &[u8], offset: usize) -> u32 {
        u32::from_ne_bytes(bytes[offset..offset + 4].try_into().unwrap())
    }

    #[test]
    fn the_name_length_excludes_the_terminator() {
        let bytes = encode::<false>(NAME);

        assert_eq!(field(&bytes, NAMELEN_OFFSET), NAME.len() as u32);
    }

    #[test]
    fn the_payload_is_the_name_and_a_terminator() {
        let bytes = encode::<false>(NAME);

        assert_eq!(&bytes[OUT_LEN..], b"a-name\0");
    }

    #[test]
    fn the_header_length_covers_the_terminator() {
        let bytes = encode::<false>(NAME);

        assert_eq!(field(&bytes, 0), (OUT_LEN + NAME.len() + 1) as u32);
        assert_eq!(bytes.len(), OUT_LEN + NAME.len() + 1);
    }

    #[test]
    fn an_empty_name_is_just_a_terminator() {
        let bytes = encode::<false>(b"");

        assert_eq!(field(&bytes, NAMELEN_OFFSET), 0);
        assert_eq!(field(&bytes, 0), (OUT_LEN + 1) as u32);
        assert_eq!(&bytes[OUT_LEN..], b"\0");
    }

    #[test]
    fn invalidating_and_expiring_differ_only_in_the_flag() {
        let invalidate = encode::<false>(NAME);
        let expire = encode::<true>(NAME);

        assert_eq!(field(&invalidate, 4), NotifyCode::INVAL_ENTRY.0);
        assert_eq!(field(&expire, 4), NotifyCode::INVAL_ENTRY.0);
        assert_eq!(field(&invalidate, FLAGS_OFFSET), 0);
        assert_eq!(field(&expire, FLAGS_OFFSET), 1);
    }
}
