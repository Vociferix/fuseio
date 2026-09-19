use super::Req;
use crate::proto::request::MkDir;
use crate::types::{Ino, Mode};

use std::ffi::OsStr;

#[derive(Debug)]
pub struct MakeDirReq {
    req: Req,
    mkdir: MkDir,
}

impl MakeDirReq {
    pub(crate) fn new(req: Req, mkdir: MkDir) -> Self {
        Self { req, mkdir }
    }

    pub fn parent(&self) -> Ino {
        self.mkdir.parent()
    }

    pub fn mode(&self) -> Mode {
        self.mkdir.mode()
    }

    pub fn umask(&self) -> Mode {
        self.mkdir.umask()
    }

    pub fn name(&self) -> &OsStr {
        self.mkdir.name()
    }
}

impl std::ops::Deref for MakeDirReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
