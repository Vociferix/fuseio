use super::{Cfg, Ino};
use crate::{Error, Result, buf::Buf};

#[derive(Debug)]
pub struct SyncFs {
    ino: Ino,
}

impl SyncFs {
    pub fn ino(&self) -> Ino {
        self.ino
    }
}

impl SyncFs {
    pub(super) fn decode(_: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        Ok(Self { ino })
    }
}
