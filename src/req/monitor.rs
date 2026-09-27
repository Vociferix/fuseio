use super::Req;
use crate::proto::request::Monitor;
use crate::types::Ino;

#[derive(Debug)]
pub struct MonitorReq<C> {
    req: Req<C>,
    monitor: Monitor,
}

impl<C> MonitorReq<C> {
    pub(crate) fn new(req: Req<C>, monitor: Monitor) -> Self {
        Self { req, monitor }
    }

    pub fn req(&self) -> &Req<C> {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.monitor.ino()
    }

    pub fn begin(&self) -> bool {
        self.monitor.begin()
    }

    pub fn end(&self) -> bool {
        self.monitor.end()
    }
}

impl<C> std::ops::Deref for MonitorReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
