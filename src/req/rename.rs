use super::Req;
use crate::proto::request::Rename;
use crate::types::{Ino, RenameMode};

use std::ffi::OsStr;

#[derive(Debug)]
pub struct RenameReq<C> {
    req: Req<C>,
    rename: Rename,
}

impl<C> RenameReq<C> {
    pub(crate) fn new(req: Req<C>, rename: Rename) -> Self {
        Self { req, rename }
    }

    pub fn req(&self) -> &Req<C> {
        &self.req
    }

    pub fn old_parent(&self) -> Ino {
        self.rename.old_parent()
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

impl<C> std::ops::Deref for RenameReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
