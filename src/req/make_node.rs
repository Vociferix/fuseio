use super::Req;
use crate::proto::request::MkNod;
use crate::types::{Ino, InodeKind, Mode};

use std::ffi::OsStr;

#[derive(Debug)]
pub struct MakeNodeReq {
    req: Req,
    mknod: MkNod,
}

impl MakeNodeReq {
    pub(crate) fn new(req: Req, mknod: MkNod) -> Self {
        Self { req, mknod }
    }

    pub fn parent(&self) -> Ino {
        self.mknod.parent()
    }

    pub fn mode(&self) -> Mode {
        self.mknod.mode()
    }

    pub fn umask(&self) -> Mode {
        self.mknod.umask()
    }

    pub fn kind(&self) -> InodeKind {
        self.mknod.kind()
    }

    pub fn rdev(&self) -> Option<u32> {
        self.mknod.rdev()
    }

    pub fn name(&self) -> &OsStr {
        self.mknod.name()
    }
}

impl std::ops::Deref for MakeNodeReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
