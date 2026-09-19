use super::Req;
use crate::proto::request::Rename;
use crate::types::{Ino, RenameMode};

use std::ffi::OsStr;

#[derive(Debug)]
pub struct RenameReq<'a> {
    req: Req<'a>,
    rename: Rename,
}

impl<'a> RenameReq<'a> {
    pub(crate) fn new(req: Req<'a>, rename: Rename) -> Self {
        Self { req, rename }
    }

    pub fn ino(&self) -> Ino {
        self.rename.ino()
    }

    pub fn new_parent(&self) -> Ino {
        self.rename.new_parent()
    }

    pub fn old_name(&self) -> &OsStr {
        self.rename.old_name()
    }

    pub fn new_name(&self) -> &OsStr {
        self.rename.new_name()
    }

    pub fn rename_mode(&self) -> RenameMode {
        self.rename.rename_mode()
    }
}

impl<'a> std::ops::Deref for RenameReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
