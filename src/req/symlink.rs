use super::Req;
use crate::proto::request::Symlink;
use crate::types::Ino;

use std::ffi::OsStr;
use std::path::Path;

#[derive(Debug)]
pub struct SymlinkReq {
    req: Req,
    symlink: Symlink,
}

impl SymlinkReq {
    pub(crate) fn new(req: Req, symlink: Symlink) -> Self {
        Self { req, symlink }
    }

    pub fn req(&self) -> &Req {
        &self.req
    }

    pub fn parent(&self) -> Ino {
        self.symlink.parent()
    }

    pub fn name(&self) -> &OsStr {
        self.symlink.name()
    }

    pub fn link(&self) -> &Path {
        self.symlink.link()
    }
}

impl std::ops::Deref for SymlinkReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
