use super::Req;
use crate::proto::request::SyncFs;
use crate::types::Ino;

#[derive(Debug)]
pub struct SyncFsReq {
    req: Req,
    syncfs: SyncFs,
}

impl SyncFsReq {
    pub(crate) fn new(req: Req, syncfs: SyncFs) -> Self {
        Self { req, syncfs }
    }

    pub fn req(&self) -> &Req {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.syncfs.ino()
    }
}

impl std::ops::Deref for SyncFsReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
