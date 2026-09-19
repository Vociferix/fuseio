use super::Req;
use crate::proto::request::Flush;
use crate::types::{FileHandle, Ino, LockOwner};

#[derive(Debug)]
pub struct FlushReq<'a> {
    req: Req<'a>,
    flush: Flush,
}

impl<'a> FlushReq<'a> {
    pub(crate) fn new(req: Req<'a>, flush: Flush) -> Self {
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

impl<'a> std::ops::Deref for FlushReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
