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
