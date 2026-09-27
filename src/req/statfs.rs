use super::Req;
use crate::proto::request::StatFs;
use crate::types::Ino;

#[derive(Debug)]
pub struct StatFsReq<C> {
    req: Req<C>,
    statfs: StatFs,
}

impl<C> StatFsReq<C> {
    pub(crate) fn new(req: Req<C>, statfs: StatFs) -> Self {
        Self { req, statfs }
    }

    pub fn req(&self) -> &Req<C> {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.statfs.ino()
    }
}

impl<C> std::ops::Deref for StatFsReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
