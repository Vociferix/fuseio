use super::Req;
use crate::proto::request::StatFs;
use crate::types::Ino;

#[derive(Debug)]
pub struct StatFsReq<'a> {
    req: Req<'a>,
    statfs: StatFs,
}

impl<'a> StatFsReq<'a> {
    pub(crate) fn new(req: Req<'a>, statfs: StatFs) -> Self {
        Self { req, statfs }
    }

    pub fn ino(&self) -> Ino {
        self.statfs.ino()
    }
}

impl<'a> std::ops::Deref for StatFsReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
