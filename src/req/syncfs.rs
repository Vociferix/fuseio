use super::Req;
use crate::proto::request::SyncFs;
use crate::types::Ino;

#[derive(Debug)]
pub struct SyncFsReq<'a> {
    req: Req<'a>,
    syncfs: SyncFs,
}

impl<'a> SyncFsReq<'a> {
    pub(crate) fn new(req: Req<'a>, syncfs: SyncFs) -> Self {
        Self { req, syncfs }
    }

    pub fn ino(&self) -> Ino {
        self.syncfs.ino()
    }
}

impl<'a> std::ops::Deref for SyncFsReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
