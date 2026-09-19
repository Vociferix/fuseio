use super::Req;
use crate::proto::request::Read;
use crate::types::{FileHandle, Ino, LockOwner, OFlag};

#[derive(Debug)]
pub struct ReadReq {
    req: Req,
    read: Read,
}

impl ReadReq {
    pub(crate) fn new(req: Req, read: Read) -> Self {
        Self { req, read }
    }

    pub fn req(&self) -> &Req {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.read.ino()
    }

    pub fn file_handle(&self) -> FileHandle {
        self.read.file_handle()
    }

    pub fn offset(&self) -> u64 {
        self.read.offset()
    }

    pub fn len(&self) -> usize {
        self.read.len()
    }

    pub fn lock_owner(&self) -> Option<LockOwner> {
        self.read.lock_owner()
    }

    pub fn open_flags(&self) -> OFlag {
        self.read.open_flags()
    }
}

impl std::ops::Deref for ReadReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
