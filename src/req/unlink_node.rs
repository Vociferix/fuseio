use super::Req;
use crate::proto::request::Unlink;
use crate::types::Ino;

use std::ffi::OsStr;

#[derive(Debug)]
pub struct UnlinkNodeReq<'a> {
    req: Req<'a>,
    unlink: Unlink,
}

impl<'a> UnlinkNodeReq<'a> {
    pub(crate) fn new(req: Req<'a>, unlink: Unlink) -> Self {
        Self { req, unlink }
    }

    pub fn ino(&self) -> Ino {
        self.unlink.ino()
    }

    pub fn name(&self) -> &OsStr {
        self.unlink.name()
    }
}

impl<'a> std::ops::Deref for UnlinkNodeReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
