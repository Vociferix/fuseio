use super::Req;
use crate::proto::request::SetVolName;

use std::ffi::OsStr;

#[derive(Debug)]
pub struct SetVolumeNameReq<C> {
    req: Req<C>,
    setvolname: SetVolName,
}

impl<C> SetVolumeNameReq<C> {
    pub(crate) fn new(req: Req<C>, setvolname: SetVolName) -> Self {
        Self { req, setvolname }
    }

    pub fn req(&self) -> &Req<C> {
        &self.req
    }

    pub fn volume_name(&self) -> &OsStr {
        self.setvolname.volume_name()
    }
}

impl<C> std::ops::Deref for SetVolumeNameReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
