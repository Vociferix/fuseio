use super::{Cfg, EncodeResp, IntoIoBuf, RawHeader};
use crate::Vectored;

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

        Ok(Vectored((hdr, buf.into_vectored())))
    }
}
