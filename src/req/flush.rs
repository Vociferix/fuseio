use super::Req;
use crate::proto::request::Flush;
use crate::types::{FileHandle, Ino, LockOwner};

#[derive(Debug)]
pub struct FlushReq {
    req: Req,
    flush: Flush,
}

impl FlushReq {
    pub(crate) fn new(req: Req, flush: Flush) -> Self {
        Self { req, flush }
    }

    pub fn ino(&self) -> Ino {
        self.flush.ino()
    }

    pub fn file_handle(&self) -> FileHandle {
        self.flush.file_handle()
    }

    pub fn lock_owner(&self) -> Option<LockOwner> {
        self.flush.lock_owner()
    }
}

impl std::ops::Deref for FlushReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
