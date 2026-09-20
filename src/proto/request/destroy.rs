use super::{Cfg, Ino};
use crate::{Result, buf::Buf};

#[derive(Debug)]
pub struct Destroy {
    _priv: (),
}

impl Destroy {
    pub(super) fn decode(_: Buf, _: Option<Ino>, _: Cfg) -> Result<Self> {
        Ok(Self { _priv: () })
    }
}
