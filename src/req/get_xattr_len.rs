use super::Req;
use crate::proto::request::GetXattr;
use crate::types::Ino;

use std::ffi::OsStr;

#[derive(Debug)]
pub struct GetXattrLenReq {
    req: Req,
    getxattr: GetXattr,
}

impl GetXattrLenReq {
    pub(crate) fn new(req: Req, getxattr: GetXattr) -> Self {
        Self { req, getxattr }
    }

    pub fn req(&self) -> &Req {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.getxattr.ino()
    }

    pub fn key(&self) -> &OsStr {
        self.getxattr.key()
    }
}

impl std::ops::Deref for GetXattrLenReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
