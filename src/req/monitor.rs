use super::Req;
use crate::proto::request::Monitor;
use crate::types::Ino;

#[derive(Debug)]
pub struct MonitorReq {
    req: Req,
    monitor: Monitor,
}

impl MonitorReq {
    pub(crate) fn new(req: Req, monitor: Monitor) -> Self {
        Self { req, monitor }
    }

    pub fn req(&self) -> &Req {
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

impl std::ops::Deref for MonitorReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
