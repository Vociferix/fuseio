use super::Req;
use crate::proto::request::GetXtimes;
use crate::types::Ino;

#[derive(Debug)]
pub struct GetXTimesReq {
    req: Req,
    getxtimes: GetXtimes,
}

impl GetXTimesReq {
    pub(crate) fn new(req: Req, getxtimes: GetXtimes) -> Self {
        Self { req, getxtimes }
    }

    pub fn req(&self) -> &Req {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.getxtimes.ino()
    }
}

impl std::ops::Deref for GetXTimesReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
