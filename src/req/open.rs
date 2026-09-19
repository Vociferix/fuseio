use super::Req;
use crate::proto::request::Open;
use crate::types::{Ino, OFlag, OpenFlags};

#[derive(Debug)]
pub struct OpenReq<'a> {
    req: Req<'a>,
    open: Open,
}

impl<'a> OpenReq<'a> {
    pub(crate) fn new(req: Req<'a>, open: Open) -> Self {
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

impl<'a> std::ops::Deref for OpenReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
