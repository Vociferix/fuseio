use super::Req;
use crate::proto::request::Write;
use crate::types::{FileHandle, Ino, LockOwner, OFlag};

#[derive(Debug)]
pub struct WriteReq<C> {
    req: Req<C>,
    write: Write,
}

impl<C> WriteReq<C> {
    pub(crate) fn new(req: Req<C>, write: Write) -> Self {
        Self { req, write }
    }

    pub fn req(&self) -> &Req<C> {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.write.ino()
    }

    pub fn file_handle(&self) -> FileHandle {
        self.write.file_handle()
    }

    pub fn offset(&self) -> u64 {
        self.write.offset()
    }

    pub fn data(&self) -> &[u8] {
        self.write.data()
    }

    pub fn lock_owner(&self) -> Option<LockOwner> {
        self.write.lock_owner()
    }

    pub fn open_flags(&self) -> OFlag {
        self.write.open_flags()
    }

    pub fn cache_writeback(&self) -> bool {
        self.write.cache_writeback()
    }

    pub fn remove_suid_sgid(&self) -> bool {
        self.write.remove_suid_sgid()
    }
}

impl<C> std::ops::Deref for WriteReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
