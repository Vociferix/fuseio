use super::Req;
use crate::proto::request::GetXattr;
use crate::types::Ino;

use std::ffi::OsStr;

#[derive(Debug)]
pub struct GetXattrReq {
    req: Req,
    getxattr: GetXattr,
}

impl GetXattrReq {
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

    pub fn len(&self) -> usize {
        self.getxattr.len()
    }

    pub fn offset(&self) -> usize {
        self.getxattr.offset()
    }
}

impl std::ops::Deref for GetXattrReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
