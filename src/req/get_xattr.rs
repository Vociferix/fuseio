use super::Req;
use crate::proto::request::GetXattr;
use crate::types::Ino;

use std::ffi::OsStr;

#[derive(Debug)]
pub struct GetXattrReq<'a> {
    req: Req<'a>,
    getxattr: GetXattr,
}

impl<'a> GetXattrReq<'a> {
    pub(crate) fn new(req: Req<'a>, getxattr: GetXattr) -> Self {
        Self { req, getxattr }
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

impl<'a> std::ops::Deref for GetXattrReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
