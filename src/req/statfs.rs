use super::Req;
use crate::proto::request::StatFs;
use crate::types::Ino;

#[derive(Debug)]
pub struct StatFsReq {
    req: Req,
    statfs: StatFs,
}

impl StatFsReq {
    pub(crate) fn new(req: Req, statfs: StatFs) -> Self {
        Self { req, statfs }
    }

    pub fn req(&self) -> &Req {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.statfs.ino()
    }
}

impl std::ops::Deref for StatFsReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
