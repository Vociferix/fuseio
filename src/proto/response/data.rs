use super::{Cfg, EncodeResp, IntoIoBuf, RawHeader};
use crate::buf::{Pod, Vectored};

pub struct Data<T> {
    data: T,
}

impl<T: IntoIoBuf> Data<T> {
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

impl<T: IntoIoBuf> From<T> for Data<T> {
    fn from(data: T) -> Self {
        Self { data }
    }
}

impl<T: IntoIoBuf> EncodeResp for Data<T> {
    type Error = crate::Error;

    fn encode(self, id: u64, _: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        let buf = self.data.into_io_buf();
        let Some(len) = buf
            .total_len()
            .checked_add(std::mem::size_of::<RawHeader>())
        else {
            return Err(crate::Error::ERANGE);
        };
        let Ok(len) = u32::try_from(len) else {
            return Err(crate::Error::ERANGE);
        };
        let hdr = RawHeader { len, err: 0, id };

        Ok(Vectored((Pod(hdr), buf.into_vectored())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::response::test_support::{
        HDR, assert_reply_header, cfg, encode, encode_with,
    };

    #[test]
    fn the_payload_follows_the_header() {
        let bytes = encode(Data::new(b"contents".to_vec()), 5);

        assert_reply_header(&bytes, 5);
        assert_eq!(&bytes[HDR..], b"contents");
    }

    // A read that found nothing is still a reply, and its length still counts
    // the header.
    #[test]
    fn an_empty_payload_is_a_header_alone() {
        let bytes = encode(Data::new(Vec::new()), 5);

        assert_eq!(bytes.len(), HDR);
        assert_reply_header(&bytes, 5);
    }

    // A reply assembled from several pieces arrives as one message, since the
    // kernel reads it as one.
    #[test]
    fn a_gathered_payload_arrives_as_one_message() {
        let pieces = crate::buf::Vectored(vec![b"ab".to_vec(), b"cd".to_vec()]);
        let bytes = encode(Data::new(pieces), 6);

        assert_reply_header(&bytes, 6);
        assert_eq!(&bytes[HDR..], b"abcd");
    }

    // `len` is a `u32`, so a payload that cannot be described is refused rather
    // than truncated into a length the kernel would read as a short reply.
    #[test]
    fn a_payload_too_large_to_describe_is_refused() {
        struct TooLong;

        impl compio::buf::IoBuf for TooLong {
            fn as_init(&self) -> &[u8] {
                b""
            }

            fn buf_len(&self) -> usize {
                u32::MAX as usize
            }
        }

        let err = Data::new(TooLong).encode(1, cfg()).map(|_| ()).unwrap_err();

        assert_eq!(err, crate::Error::ERANGE);
        let _ = encode_with::<Data<Vec<u8>>>;
    }
}
