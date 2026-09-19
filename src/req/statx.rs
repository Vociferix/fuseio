use super::Req;
use crate::proto::request::StatX;
use crate::types::{FileHandle, Ino, StatXMask, StatXSync};

#[derive(Debug)]
pub struct StatXReq {
    req: Req,
    statx: StatX,
}

impl StatXReq {
    pub(crate) fn new(req: Req, statx: StatX) -> Self {
        Self { req, statx }
    }

    pub fn req(&self) -> &Req {
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

impl std::ops::Deref for StatXReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
