use super::Req;
use crate::proto::request::Lseek;
use crate::types::{FileHandle, Ino, Whence};

#[derive(Debug)]
pub struct LseekReq<C> {
    req: Req<C>,
    lseek: Lseek,
}

impl<C> LseekReq<C> {
    pub(crate) fn new(req: Req<C>, lseek: Lseek) -> Self {
        Self { req, lseek }
    }

    pub fn req(&self) -> &Req<C> {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.lseek.ino()
    }

    pub fn file_handle(&self) -> FileHandle {
        self.lseek.file_handle()
    }

    pub fn offset(&self) -> u64 {
        self.lseek.offset()
    }

    pub fn whence(&self) -> Whence {
        self.lseek.whence()
    }
}

impl<C> std::ops::Deref for LseekReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
