use super::Req;
use crate::proto::request::SetVolName;

use std::ffi::OsStr;

#[derive(Debug)]
pub struct SetVolumeNameReq {
    req: Req,
    setvolname: SetVolName,
}

impl SetVolumeNameReq {
    pub(crate) fn new(req: Req, setvolname: SetVolName) -> Self {
        Self { req, setvolname }
    }

    pub fn req(&self) -> &Req {
        &self.req
    }

    pub fn volume_name(&self) -> &OsStr {
        self.setvolname.volume_name()
    }
}

impl std::ops::Deref for SetVolumeNameReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
