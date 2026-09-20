use super::{Cfg, Ino};
use crate::{Error, Result, buf::Buf};

#[derive(Debug)]
pub struct GetXtimes {
    ino: Ino,
}

impl GetXtimes {
    pub fn ino(&self) -> Ino {
        self.ino
    }
}

impl GetXtimes {
    pub(super) fn decode(_: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        Ok(Self { ino })
    }
}
