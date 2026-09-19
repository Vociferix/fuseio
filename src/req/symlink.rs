use super::Req;
use crate::proto::request::Symlink;
use crate::types::Ino;

use std::ffi::OsStr;
use std::path::Path;

#[derive(Debug)]
pub struct SymlinkReq<'a> {
    req: Req<'a>,
    symlink: Symlink,
}

impl<'a> SymlinkReq<'a> {
    pub(crate) fn new(req: Req<'a>, symlink: Symlink) -> Self {
        Self { req, symlink }
    }

    pub fn ino(&self) -> Ino {
        self.symlink.ino()
    }

    pub fn name(&self) -> &OsStr {
        self.symlink.name()
    }

    pub fn link(&self) -> &Path {
        self.symlink.link()
    }
}

impl<'a> std::ops::Deref for SymlinkReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
