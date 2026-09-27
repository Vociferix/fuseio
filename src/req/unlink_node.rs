use super::Req;
use crate::proto::request::Unlink;
use crate::types::Ino;

use std::ffi::OsStr;

#[derive(Debug)]
pub struct UnlinkNodeReq<C> {
    req: Req<C>,
    unlink: Unlink,
}

impl<C> UnlinkNodeReq<C> {
    pub(crate) fn new(req: Req<C>, unlink: Unlink) -> Self {
        Self { req, unlink }
    }

    pub fn req(&self) -> &Req<C> {
        &self.req
    }

    pub fn parent(&self) -> Ino {
        self.unlink.parent()
    }

    pub fn name(&self) -> &OsStr {
        self.unlink.name()
    }
}

impl<C> std::ops::Deref for UnlinkNodeReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
