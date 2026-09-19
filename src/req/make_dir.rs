use super::Req;
use crate::proto::request::MkDir;
use crate::types::{Ino, Mode};

use std::ffi::OsStr;

#[derive(Debug)]
pub struct MakeDirReq<'a> {
    req: Req<'a>,
    mkdir: MkDir,
}

impl<'a> MakeDirReq<'a> {
    pub(crate) fn new(req: Req<'a>, mkdir: MkDir) -> Self {
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

impl<'a> std::ops::Deref for MakeDirReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
