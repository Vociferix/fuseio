use super::Req;
use crate::proto::request::Fallocate;
use crate::types::{FallocateFlags, FileHandle, Ino};

#[derive(Debug)]
pub struct FallocateReq<'a> {
    req: Req<'a>,
    fallocate: Fallocate,
}

impl<'a> FallocateReq<'a> {
    pub(crate) fn new(req: Req<'a>, fallocate: Fallocate) -> Self {
        Self { req, fallocate }
    }

    pub fn ino(&self) -> Ino {
        self.fallocate.ino()
    }

    pub fn file_handle(&self) -> FileHandle {
        self.fallocate.file_handle()
    }

    pub fn offset(&self) -> u64 {
        self.fallocate.offset()
    }

    pub fn len(&self) -> u64 {
        self.fallocate.len()
    }

    pub fn flags(&self) -> FallocateFlags {
        self.fallocate.flags()
    }
}

impl<'a> std::ops::Deref for FallocateReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
