use super::Req;
use crate::proto::request::Fallocate;
use crate::types::{FallocateFlags, FileHandle, Ino};

#[derive(Debug)]
pub struct FallocateReq<C> {
    req: Req<C>,
    fallocate: Fallocate,
}

impl<C> FallocateReq<C> {
    pub(crate) fn new(req: Req<C>, fallocate: Fallocate) -> Self {
        Self { req, fallocate }
    }

    pub fn req(&self) -> &Req<C> {
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

    // Not a collection: this is a byte count the kernel asked for, and a zero
    // one is meaningful rather than "empty".
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> u64 {
        self.fallocate.len()
    }

    pub fn flags(&self) -> FallocateFlags {
        self.fallocate.flags()
    }
}

impl<C> std::ops::Deref for FallocateReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
