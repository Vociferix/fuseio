use super::Req;
use crate::proto::request::RmDir;
use crate::types::Ino;

use std::ffi::OsStr;

#[derive(Debug)]
pub struct RemoveDirReq<'a> {
    req: Req<'a>,
    rmdir: RmDir,
}

impl<'a> RemoveDirReq<'a> {
    pub(crate) fn new(req: Req<'a>, rmdir: RmDir) -> Self {
        Self { req, rmdir }
    }

    pub fn ino(&self) -> Ino {
        self.rmdir.ino()
    }

    pub fn name(&self) -> &OsStr {
        self.rmdir.name()
    }
}

impl<'a> std::ops::Deref for RemoveDirReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
