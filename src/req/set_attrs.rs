use super::Req;
use crate::proto::request::SetAttr;
use crate::types::{FileFlag, FileHandle, FileTime, Gid, Ino, Mode, Uid};

use std::time::SystemTime;

#[derive(Debug)]
pub struct SetAttrsReq {
    req: Req,
    set_attr: SetAttr,
}

impl SetAttrsReq {
    pub(crate) fn new(req: Req, set_attr: SetAttr) -> Self {
        Self { req, set_attr }
    }

    pub fn req(&self) -> &Req {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.set_attr.ino()
    }

    pub fn file_handle(&self) -> Option<FileHandle> {
        self.set_attr.file_handle()
    }

    pub fn size(&self) -> Option<u64> {
        self.set_attr.size()
    }

    pub fn atime(&self) -> Option<FileTime> {
        self.set_attr.atime()
    }

    pub fn mtime(&self) -> Option<FileTime> {
        self.set_attr.mtime()
    }

    pub fn ctime(&self) -> Option<SystemTime> {
        self.set_attr.ctime()
    }

    pub fn bkuptime(&self) -> Option<SystemTime> {
        self.set_attr.bkuptime()
    }

    pub fn crtime(&self) -> Option<SystemTime> {
        self.set_attr.crtime()
    }

    pub fn mode(&self) -> Option<Mode> {
        self.set_attr.mode()
    }

    pub fn uid(&self) -> Option<Uid> {
        self.set_attr.uid()
    }

    pub fn gid(&self) -> Option<Gid> {
        self.set_attr.gid()
    }

    pub fn remove_suid_sgid(&self) -> bool {
        self.set_attr.remove_suid_sgid()
    }

    pub fn flags(&self) -> Option<FileFlag> {
        self.set_attr.flags()
    }
}

impl std::ops::Deref for SetAttrsReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
