use super::Req;
use crate::proto::request::TmpFile;
use crate::types::{Ino, Mode, OFlag, OpenFlags};

#[derive(Debug)]
pub struct TmpFileReq {
    req: Req,
    create: TmpFile,
}

impl TmpFileReq {
    pub(crate) fn new(req: Req, create: TmpFile) -> Self {
        Self { req, create }
    }

    pub fn req(&self) -> &Req {
        &self.req
    }

    pub fn parent(&self) -> Ino {
        self.create.parent()
    }

    pub fn mode(&self) -> Mode {
        self.create.mode()
    }

    pub fn umask(&self) -> Mode {
        self.create.umask()
    }

    pub fn flags(&self) -> OFlag {
        self.create.flags()
    }

    pub fn op_flags(&self) -> OpenFlags {
        self.create.op_flags()
    }
}

impl std::ops::Deref for TmpFileReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
