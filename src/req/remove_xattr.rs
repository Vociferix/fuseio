use super::Req;
use crate::proto::request::RemoveXattr;
use crate::types::Ino;

use std::ffi::OsStr;

#[derive(Debug)]
pub struct RemoveXattrReq<'a> {
    req: Req<'a>,
    removexattr: RemoveXattr,
}

impl<'a> RemoveXattrReq<'a> {
    pub(crate) fn new(req: Req<'a>, removexattr: RemoveXattr) -> Self {
        Self { req, removexattr }
    }

    pub fn ino(&self) -> Ino {
        self.removexattr.ino()
    }

    pub fn key(&self) -> &OsStr {
        self.removexattr.key()
    }
}

impl<'a> std::ops::Deref for RemoveXattrReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
