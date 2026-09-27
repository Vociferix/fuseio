use super::Req;
use crate::proto::request::RemoveXattr;
use crate::types::Ino;

use std::ffi::OsStr;

#[derive(Debug)]
pub struct RemoveXattrReq<C> {
    req: Req<C>,
    removexattr: RemoveXattr,
}

impl<C> RemoveXattrReq<C> {
    pub(crate) fn new(req: Req<C>, removexattr: RemoveXattr) -> Self {
        Self { req, removexattr }
    }

    pub fn req(&self) -> &Req<C> {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.removexattr.ino()
    }

    pub fn key(&self) -> &OsStr {
        self.removexattr.key()
    }
}

impl<C> std::ops::Deref for RemoveXattrReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
