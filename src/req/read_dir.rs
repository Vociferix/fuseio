use super::Req;
use crate::proto::request::ReadDir;
use crate::types::{FileHandle, Ino, LockOwner, OFlag};

#[derive(Debug)]
pub struct ReadDirReq<'a> {
    req: Req<'a>,
    readdir: ReadDir,
}

impl<'a> ReadDirReq<'a> {
    pub(crate) fn new(req: Req<'a>, readdir: ReadDir) -> Self {
        Self { req, readdir }
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

impl<'a> std::ops::Deref for ReadDirReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
