use super::Req;
use crate::proto::request::SyncFs;
use crate::types::Ino;

#[derive(Debug)]
pub struct SyncFsReq<C> {
    req: Req<C>,
    syncfs: SyncFs,
}

impl<C> SyncFsReq<C> {
    pub(crate) fn new(req: Req<C>, syncfs: SyncFs) -> Self {
        Self { req, syncfs }
    }

    pub fn req(&self) -> &Req<C> {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.syncfs.ino()
    }
}

impl<C> std::ops::Deref for SyncFsReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
