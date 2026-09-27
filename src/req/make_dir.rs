use super::Req;
use crate::proto::request::MkDir;
use crate::types::{Ino, Mode};

use std::ffi::OsStr;

#[derive(Debug)]
pub struct MakeDirReq<C> {
    req: Req<C>,
    mkdir: MkDir,
}

impl<C> MakeDirReq<C> {
    pub(crate) fn new(req: Req<C>, mkdir: MkDir) -> Self {
        Self { req, mkdir }
    }

    pub fn req(&self) -> &Req<C> {
        &self.req
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

impl<C> std::ops::Deref for MakeDirReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
