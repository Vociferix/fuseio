use super::{Cfg, Ino};
use crate::{Buf, Result};

#[derive(Debug)]
pub struct Destroy {
    _priv: (),
}

impl Destroy {
    pub(super) fn decode(_: Buf, _: Option<Ino>, _: Cfg) -> Result<Self> {
        Ok(Self { _priv: () })
    }
}
