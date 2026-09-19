use super::Req;
use crate::proto::request::Read;
use crate::types::{FileHandle, Ino, LockOwner, OFlag};

#[derive(Debug)]
pub struct ReadReq<'a> {
    req: Req<'a>,
    read: Read,
}

impl<'a> ReadReq<'a> {
    pub(crate) fn new(req: Req<'a>, read: Read) -> Self {
        Self { req, read }
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

impl<'a> std::ops::Deref for ReadReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
