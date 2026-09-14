use super::{Cfg, EncodeNotify, IntoIoBuf, NotifyCode, RawHeader};

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
        Ok(RawHeader {
            len: const { std::mem::size_of::<RawHeader>() as u32 },
            code: NotifyCode::INC_EPOCH,
            _zero: 0,
        })
    }
}
