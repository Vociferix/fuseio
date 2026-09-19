use super::Req;
use crate::proto::request::MkNod;
use crate::types::{Ino, InodeKind, Mode};

use std::ffi::OsStr;

#[derive(Debug)]
pub struct MakeNodeReq<'a> {
    req: Req<'a>,
    mknod: MkNod,
}

impl<'a> MakeNodeReq<'a> {
    pub(crate) fn new(req: Req<'a>, mknod: MkNod) -> Self {
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

impl<'a> std::ops::Deref for MakeNodeReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
