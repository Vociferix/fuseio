use super::Req;
use crate::proto::request::Fsync;
use crate::types::{FileHandle, Ino};

#[derive(Debug)]
pub struct FsyncReq<C> {
    req: Req<C>,
    fsync: Fsync,
}

impl<C> FsyncReq<C> {
    pub(crate) fn new(req: Req<C>, fsync: Fsync) -> Self {
        Self { req, fsync }
    }

    pub fn req(&self) -> &Req<C> {
        &self.req
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

impl<C> std::ops::Deref for FsyncReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
