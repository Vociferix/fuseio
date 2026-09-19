use super::Req;
use crate::proto::request::ReadDir;
use crate::types::{FileHandle, Ino, LockOwner, OFlag};

#[derive(Debug)]
pub struct ReadDirReq {
    req: Req,
    readdir: ReadDir,
}

impl ReadDirReq {
    pub(crate) fn new(req: Req, readdir: ReadDir) -> Self {
        Self { req, readdir }
    }

    pub fn req(&self) -> &Req {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.readdir.ino()
    }

    pub fn file_handle(&self) -> FileHandle {
        self.readdir.file_handle()
    }

    pub fn offset(&self) -> u64 {
        self.readdir.offset()
    }

    pub fn len(&self) -> usize {
        self.readdir.len()
    }

    pub fn lock_owner(&self) -> Option<LockOwner> {
        self.readdir.lock_owner()
    }

    pub fn open_flags(&self) -> OFlag {
        self.readdir.open_flags()
    }
}

impl std::ops::Deref for ReadDirReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
