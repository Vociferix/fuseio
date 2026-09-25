use super::Req;
use crate::proto::request::Unlink;
use crate::types::Ino;

use std::ffi::OsStr;

#[derive(Debug)]
pub struct UnlinkNodeReq {
    req: Req,
    unlink: Unlink,
}

impl UnlinkNodeReq {
    pub(crate) fn new(req: Req, unlink: Unlink) -> Self {
        Self { req, unlink }
    }

    pub fn req(&self) -> &Req {
        &self.req
    }

    pub fn parent(&self) -> Ino {
        self.unlink.parent()
    }

    pub fn name(&self) -> &OsStr {
        self.unlink.name()
    }
}

impl std::ops::Deref for UnlinkNodeReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
