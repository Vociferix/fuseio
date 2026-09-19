use super::Req;
use crate::proto::request::StatX;
use crate::types::{FileHandle, Ino, StatXMask, StatXSync};

#[derive(Debug)]
pub struct StatXReq<'a> {
    req: Req<'a>,
    statx: StatX,
}

impl<'a> StatXReq<'a> {
    pub(crate) fn new(req: Req<'a>, statx: StatX) -> Self {
        Self { req, statx }
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

impl<'a> std::ops::Deref for StatXReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
