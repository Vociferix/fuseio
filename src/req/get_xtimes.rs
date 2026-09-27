use super::Req;
use crate::proto::request::GetXtimes;
use crate::types::Ino;

#[derive(Debug)]
pub struct GetXTimesReq<C> {
    req: Req<C>,
    getxtimes: GetXtimes,
}

impl<C> GetXTimesReq<C> {
    pub(crate) fn new(req: Req<C>, getxtimes: GetXtimes) -> Self {
        Self { req, getxtimes }
    }

    pub fn req(&self) -> &Req<C> {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.getxtimes.ino()
    }
}

impl<C> std::ops::Deref for GetXTimesReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
