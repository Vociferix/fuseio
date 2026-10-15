use super::{Cfg, EncodeNotify, IntoIoBuf, NotifyCode, RawHeader};
use crate::buf::{Pod, Vectored};
use crate::types::Ino;

#[derive(Debug)]
pub struct Store<B> {
    ino: Ino,
    offset: u64,
    data: B,
}

impl<B: IntoIoBuf> Store<B> {
    pub fn new(ino: Ino, offset: u64, data: B) -> Self {
        Self { ino, offset, data }
    }
}

impl<B: IntoIoBuf> EncodeNotify for Store<B> {
    type Error = crate::Error;

    fn encode(self, _: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        #[derive(Clone, Copy, bytemuck::NoUninit)]
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            nodeid: u64,
            offset: u64,
            size: u32,
            _unused: u32,
        }

        let buf = self.data.into_io_buf();
        let size = buf.total_len();

        let Ok(size) = u32::try_from(size) else {
            return Err(crate::Error::ERANGE);
        };

        let Some(len) = const { std::mem::size_of::<Out>() as u32 }.checked_add(size) else {
            return Err(crate::Error::ERANGE);
        };

        Ok(Vectored((
            Pod(Out {
                hdr: RawHeader {
                    len,
                    code: NotifyCode::STORE,
                    _zero: 0,
                },
                nodeid: self.ino.as_raw(),
                offset: self.offset,
                size,
                _unused: 0,
            }),
            buf,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::notify::test_support::{HDR, assert_notify_header, encode, u32_at, u64_at};

    const NODEID: usize = HDR;
    const OFFSET: usize = HDR + 8;
    const SIZE: usize = HDR + 16;

    // `fuse_notify_store_out { nodeid, offset, size, padding }`, code 4, with
    // the data after it.
    #[test]
    fn the_data_follows_its_description() {
        let ino = Ino::from_raw(9).unwrap();
        let bytes = encode(Store::new(ino, 4096, b"payload".to_vec()));

        assert_notify_header(&bytes, 4);
        assert_eq!(u64_at(&bytes, NODEID), 9);
        assert_eq!(u64_at(&bytes, OFFSET), 4096);
        assert_eq!(u32_at(&bytes, SIZE), 7, "size counts only the data");
        assert_eq!(u32_at(&bytes, SIZE + 4), 0, "the padding must be zero");
        assert_eq!(&bytes[HDR + 24..], b"payload");
        assert_eq!(bytes.len(), HDR + 24 + 7);
    }

    #[test]
    fn storing_nothing_still_describes_a_zero_length() {
        let ino = Ino::from_raw(1).unwrap();
        let bytes = encode(Store::new(ino, 0, Vec::new()));

        assert_eq!(u32_at(&bytes, SIZE), 0);
        assert_eq!(bytes.len(), HDR + 24);
    }
}
