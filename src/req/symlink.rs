use super::Req;
use crate::proto::request::Symlink;
use crate::types::Ino;

use std::ffi::OsStr;
use std::path::Path;

#[derive(Debug)]
pub struct SymlinkReq<C> {
    req: Req<C>,
    symlink: Symlink,
}

impl<C> SymlinkReq<C> {
    pub(crate) fn new(req: Req<C>, symlink: Symlink) -> Self {
        Self { req, symlink }
    }

    pub fn req(&self) -> &Req<C> {
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

impl<C> std::ops::Deref for SymlinkReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
