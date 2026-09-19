use super::Req;
use crate::proto::request::TmpFile;
use crate::types::{Ino, Mode, OFlag, OpenFlags};

#[derive(Debug)]
pub struct TmpFileReq<'a> {
    req: Req<'a>,
    create: TmpFile,
}

impl<'a> TmpFileReq<'a> {
    pub(crate) fn new(req: Req<'a>, create: TmpFile) -> Self {
        Self { req, create }
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

impl<'a> std::ops::Deref for TmpFileReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
