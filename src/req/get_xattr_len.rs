use super::Req;
use crate::proto::request::GetXattr;
use crate::types::Ino;

use std::ffi::OsStr;

#[derive(Debug)]
pub struct GetXattrLenReq<'a> {
    req: Req<'a>,
    getxattr: GetXattr,
}

impl<'a> GetXattrLenReq<'a> {
    pub(crate) fn new(req: Req<'a>, getxattr: GetXattr) -> Self {
        Self { req, getxattr }
    }

    pub fn ino(&self) -> Ino {
        self.getxattr.ino()
    }

    pub fn key(&self) -> &OsStr {
        self.getxattr.key()
    }
}

impl<'a> std::ops::Deref for GetXattrLenReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
