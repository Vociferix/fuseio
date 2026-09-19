use super::Req;
use crate::proto::request::Lseek;
use crate::types::{FileHandle, Ino, Whence};

#[derive(Debug)]
pub struct LseekReq<'a> {
    req: Req<'a>,
    lseek: Lseek,
}

impl<'a> LseekReq<'a> {
    pub(crate) fn new(req: Req<'a>, lseek: Lseek) -> Self {
        Self { req, lseek }
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

impl<'a> std::ops::Deref for LseekReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
