use super::Req;
use crate::proto::request::SetXattr;
use crate::types::{Ino, XattrMode};

use std::ffi::OsStr;

#[derive(Debug)]
pub struct SetXattrReq<'a> {
    req: Req<'a>,
    setxattr: SetXattr,
}

impl<'a> SetXattrReq<'a> {
    pub(crate) fn new(req: Req<'a>, setxattr: SetXattr) -> Self {
        Self { req, setxattr }
    }

    pub fn ino(&self) -> Ino {
        self.setxattr.ino()
    }

    pub fn mode(&self) -> XattrMode {
        self.setxattr.mode()
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
}

impl<'a> std::ops::Deref for SetXattrReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
