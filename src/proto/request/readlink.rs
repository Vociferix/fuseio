use super::{Cfg, Ino};
use crate::{Buf, Error, Result};

#[derive(Debug)]
pub struct ReadLink {
    ino: Ino,
}

impl ReadLink {
    pub fn ino(&self) -> Ino {
        self.ino
    }
}

impl ReadLink {
    pub(super) fn decode(_: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        Ok(Self { ino })
    }
}
