use super::Req;
use crate::proto::request::Fsync;
use crate::types::{FileHandle, Ino};

#[derive(Debug)]
pub struct FsyncReq {
    req: Req,
    fsync: Fsync,
}

impl FsyncReq {
    pub(crate) fn new(req: Req, fsync: Fsync) -> Self {
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

impl std::ops::Deref for FsyncReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
