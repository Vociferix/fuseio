use super::Req;
use crate::proto::request::SetXattr;
use crate::types::{Ino, XattrMode};

use std::ffi::OsStr;

#[derive(Debug)]
pub struct SetXattrReq {
    req: Req,
    setxattr: SetXattr,
}

impl SetXattrReq {
    pub(crate) fn new(req: Req, setxattr: SetXattr) -> Self {
        Self { req, setxattr }
    }

    pub fn req(&self) -> &Req {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.setxattr.ino()
    }

    pub fn xattr_mode(&self) -> XattrMode {
        self.setxattr.xattr_mode()
    }

    pub fn remove_sgid(&self) -> bool {
        self.setxattr.remove_sgid()
    }

    pub fn key(&self) -> &OsStr {
        self.setxattr.key()
    }

    pub fn value(&self) -> &[u8] {
        self.setxattr.value()
    }

    pub fn offset(&self) -> usize {
        self.setxattr.offset()
    }
}

impl std::ops::Deref for SetXattrReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
