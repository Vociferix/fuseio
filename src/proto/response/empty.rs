use super::{Cfg, EncodeResp, IntoIoBuf, RawHeader};
use crate::buf::Pod;

pub type Empty = ();

impl EncodeResp for Empty {
    type Error = std::convert::Infallible;

    fn encode(self, id: u64, _: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        Ok(Pod(RawHeader {
            len: const { std::mem::size_of::<RawHeader>() as u32 },
            err: 0,
            id,
        }))
    }
}

#[cfg(test)]
mod tests {
    use crate::proto::response::test_support::{HDR, assert_reply_header, encode};

    // Operations with nothing to report still answer, so the kernel can
    // release the request. The whole reply is the header.
    #[test]
    fn an_empty_reply_is_only_a_header() {
        let bytes = encode((), 42);

        assert_eq!(bytes.len(), HDR);
        assert_reply_header(&bytes, 42);
    }
}
