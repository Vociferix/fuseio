use super::Req;
use crate::proto::request::SetXattr;
use crate::types::{Ino, XattrMode};

use std::ffi::OsStr;

#[derive(Debug)]
pub struct SetXattrReq<C> {
    req: Req<C>,
    setxattr: SetXattr,
}

impl<C> SetXattrReq<C> {
    pub(crate) fn new(req: Req<C>, setxattr: SetXattr) -> Self {
        Self { req, setxattr }
    }

    pub fn req(&self) -> &Req<C> {
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

impl<C> std::ops::Deref for SetXattrReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
