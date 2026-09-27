use super::Req;
use crate::proto::request::StatX;
use crate::types::{FileHandle, Ino, StatXMask, StatXSync};

#[derive(Debug)]
pub struct StatXReq<C> {
    req: Req<C>,
    statx: StatX,
}

impl<C> StatXReq<C> {
    pub(crate) fn new(req: Req<C>, statx: StatX) -> Self {
        Self { req, statx }
    }

    pub fn req(&self) -> &Req<C> {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.statx.ino()
    }

    pub fn file_handle(&self) -> Option<FileHandle> {
        self.statx.file_handle()
    }

    pub fn sync_mode(&self) -> StatXSync {
        self.statx.sync_mode()
    }

    pub fn field_mask(&self) -> StatXMask {
        self.statx.field_mask()
    }
}

impl<C> std::ops::Deref for StatXReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
