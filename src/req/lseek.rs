use super::Req;
use crate::proto::request::Lseek;
use crate::types::{FileHandle, Ino, Whence};

#[derive(Debug)]
pub struct LseekReq {
    req: Req,
    lseek: Lseek,
}

impl LseekReq {
    pub(crate) fn new(req: Req, lseek: Lseek) -> Self {
        Self { req, lseek }
    }

    pub fn req(&self) -> &Req {
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

impl std::ops::Deref for LseekReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
