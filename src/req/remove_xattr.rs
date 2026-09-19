use super::Req;
use crate::proto::request::RemoveXattr;
use crate::types::Ino;

use std::ffi::OsStr;

#[derive(Debug)]
pub struct RemoveXattrReq {
    req: Req,
    removexattr: RemoveXattr,
}

impl RemoveXattrReq {
    pub(crate) fn new(req: Req, removexattr: RemoveXattr) -> Self {
        Self { req, removexattr }
    }

    pub fn req(&self) -> &Req {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.removexattr.ino()
    }

    pub fn key(&self) -> &OsStr {
        self.removexattr.key()
    }
}

impl std::ops::Deref for RemoveXattrReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
