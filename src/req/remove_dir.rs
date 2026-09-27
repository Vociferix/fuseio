use super::Req;
use crate::proto::request::RmDir;
use crate::types::Ino;

use std::ffi::OsStr;

#[derive(Debug)]
pub struct RemoveDirReq<C> {
    req: Req<C>,
    rmdir: RmDir,
}

impl<C> RemoveDirReq<C> {
    pub(crate) fn new(req: Req<C>, rmdir: RmDir) -> Self {
        Self { req, rmdir }
    }

    pub fn req(&self) -> &Req<C> {
        &self.req
    }

    pub fn parent(&self) -> Ino {
        self.rmdir.parent()
    }

    pub fn name(&self) -> &OsStr {
        self.rmdir.name()
    }
}

impl<C> std::ops::Deref for RemoveDirReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
