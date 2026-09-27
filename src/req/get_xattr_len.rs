use super::Req;
use crate::proto::request::GetXattr;
use crate::types::Ino;

use std::ffi::OsStr;

#[derive(Debug)]
pub struct GetXattrLenReq<C> {
    req: Req<C>,
    getxattr: GetXattr,
}

impl<C> GetXattrLenReq<C> {
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
}

impl<C> std::ops::Deref for GetXattrLenReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
