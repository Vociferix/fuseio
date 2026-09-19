use super::Req;
use crate::proto::request::Fsync;
use crate::types::{FileHandle, Ino};

#[derive(Debug)]
pub struct FsyncReq<'a> {
    req: Req<'a>,
    fsync: Fsync,
}

impl<'a> FsyncReq<'a> {
    pub(crate) fn new(req: Req<'a>, fsync: Fsync) -> Self {
        Self { req, fsync }
    }

    pub fn ino(&self) -> Ino {
        self.fsync.ino()
    }

    pub fn file_handle(&self) -> FileHandle {
        self.fsync.file_handle()
    }

    pub fn datasync(&self) -> bool {
        self.fsync.datasync()
    }
}

impl<'a> std::ops::Deref for FsyncReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
