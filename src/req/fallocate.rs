use super::Req;
use crate::proto::request::Fallocate;
use crate::types::{FallocateFlags, FileHandle, Ino};

#[derive(Debug)]
pub struct FallocateReq {
    req: Req,
    fallocate: Fallocate,
}

impl FallocateReq {
    pub(crate) fn new(req: Req, fallocate: Fallocate) -> Self {
        Self { req, fallocate }
    }

    pub fn req(&self) -> &Req {
        &self.req
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

impl std::ops::Deref for FallocateReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
