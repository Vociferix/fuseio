use super::Req;
use crate::proto::request::Open;
use crate::types::{Ino, OFlag};

#[derive(Debug)]
pub struct OpenReq {
    req: Req,
    open: Open,
}

impl OpenReq {
    pub(crate) fn new(req: Req, open: Open) -> Self {
        Self { req, open }
    }

    pub fn req(&self) -> &Req {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.open.ino()
    }

    pub fn open_flags(&self) -> OFlag {
        self.open.open_flags()
    }

    pub fn remove_suid_sgid(&self) -> bool {
        self.open.remove_suid_sgid()
    }
}

impl std::ops::Deref for OpenReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
