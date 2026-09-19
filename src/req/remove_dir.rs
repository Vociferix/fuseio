use super::Req;
use crate::proto::request::RmDir;
use crate::types::Ino;

use std::ffi::OsStr;

#[derive(Debug)]
pub struct RemoveDirReq {
    req: Req,
    rmdir: RmDir,
}

impl RemoveDirReq {
    pub(crate) fn new(req: Req, rmdir: RmDir) -> Self {
        Self { req, rmdir }
    }

    pub fn req(&self) -> &Req {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.rmdir.ino()
    }

    pub fn name(&self) -> &OsStr {
        self.rmdir.name()
    }
}

impl std::ops::Deref for RemoveDirReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
