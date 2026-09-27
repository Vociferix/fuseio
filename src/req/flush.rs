use super::Req;
use crate::proto::request::Flush;
use crate::types::{FileHandle, Ino, LockOwner};

#[derive(Debug)]
pub struct FlushReq<C> {
    req: Req<C>,
    flush: Flush,
}

impl<C> FlushReq<C> {
    pub(crate) fn new(req: Req<C>, flush: Flush) -> Self {
        Self { req, flush }
    }

    pub fn req(&self) -> &Req<C> {
        &self.req
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

impl<C> std::ops::Deref for FlushReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
