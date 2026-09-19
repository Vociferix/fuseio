use super::Req;
use crate::proto::request::Open;
use crate::types::{Ino, OFlag, OpenFlags};

#[derive(Debug)]
pub struct OpenReq {
    req: Req,
    open: Open,
}

impl OpenReq {
    pub(crate) fn new(req: Req, open: Open) -> Self {
        Self { req, open }
    }

    pub fn ino(&self) -> Ino {
        self.open.ino()
    }

    pub fn flags(&self) -> OFlag {
        self.open.flags()
    }

    pub fn op_flags(&self) -> OpenFlags {
        self.open.op_flags()
    }
}

impl std::ops::Deref for OpenReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
