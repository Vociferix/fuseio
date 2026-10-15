use super::{Cfg, EncodeNotify, IntoIoBuf, NotifyCode, RawHeader};
use crate::buf::Pod;

#[derive(Debug)]
pub struct IncrementEpoch {
    _priv: (),
}

impl IncrementEpoch {
    pub fn new() -> Self {
        Self { _priv: () }
    }
}

impl EncodeNotify for IncrementEpoch {
    type Error = std::convert::Infallible;

    fn encode(self, _: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        Ok(Pod(RawHeader {
            len: const { std::mem::size_of::<RawHeader>() as u32 },
            code: NotifyCode::INC_EPOCH,
            _zero: 0,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::proto::notify::test_support::{HDR, assert_notify_header, encode};

    // Code 8, and nothing to say beyond the code itself.
    #[test]
    fn the_notification_is_only_a_header() {
        let bytes = encode(IncrementEpoch::new());

        assert_eq!(bytes.len(), HDR);
        assert_notify_header(&bytes, 8);
    }
}
