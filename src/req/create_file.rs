use super::Req;
use crate::proto::request::Create;
use crate::types::{Ino, Mode, OFlag};

use std::ffi::OsStr;

#[derive(Debug)]
pub struct CreateFileReq {
    req: Req,
    create: Create,
}

impl CreateFileReq {
    pub(crate) fn new(req: Req, create: Create) -> Self {
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

    pub fn open_flags(&self) -> OFlag {
        self.create.open_flags()
    }

    pub fn remove_suid_sgid(&self) -> bool {
        self.create.remove_suid_sgid()
    }

    pub fn name(&self) -> &OsStr {
        self.create.name()
    }
}

impl std::ops::Deref for CreateFileReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
