use super::Req;
use crate::proto::request::TmpFile;
use crate::types::{Ino, Mode, OFlag};

#[derive(Debug)]
pub struct TmpFileReq<C> {
    req: Req<C>,
    create: TmpFile,
}

impl<C> TmpFileReq<C> {
    pub(crate) fn new(req: Req<C>, create: TmpFile) -> Self {
        Self { req, create }
    }

    pub fn req(&self) -> &Req<C> {
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

    pub fn open_flags(&self) -> OFlag {
        self.create.open_flags()
    }

    pub fn remove_suid_sgid(&self) -> bool {
        self.create.remove_suid_sgid()
    }
}

impl<C> std::ops::Deref for TmpFileReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
