use super::Req;
use crate::proto::request::Open;
use crate::types::{Ino, OFlag};

#[derive(Debug)]
pub struct OpenReq<C> {
    req: Req<C>,
    open: Open,
}

impl<C> OpenReq<C> {
    pub(crate) fn new(req: Req<C>, open: Open) -> Self {
        Self { req, open }
    }

    pub fn req(&self) -> &Req<C> {
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

impl<C> std::ops::Deref for OpenReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
