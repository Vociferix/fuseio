use super::Req;
use crate::proto::request::GetXattr;
use crate::types::Ino;

use std::ffi::OsStr;

#[derive(Debug)]
pub struct GetXattrReq<C> {
    req: Req<C>,
    getxattr: GetXattr,
}

impl<C> GetXattrReq<C> {
    pub(crate) fn new(req: Req<C>, getxattr: GetXattr) -> Self {
        Self { req, getxattr }
    }

    pub fn req(&self) -> &Req<C> {
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

impl<C> std::ops::Deref for GetXattrReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
