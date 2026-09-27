use super::Req;
use crate::proto::request::Access;
use crate::types::{AccessFlags, Ino};

#[derive(Debug)]
pub struct AccessReq<C> {
    req: Req<C>,
    access: Access,
}

impl<C> AccessReq<C> {
    pub(crate) fn new(req: Req<C>, access: Access) -> Self {
        Self { req, access }
    }

    pub fn req(&self) -> &Req<C> {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.access.ino()
    }

    pub fn flags(&self) -> AccessFlags {
        self.access.flags()
    }
}

impl<C> std::ops::Deref for AccessReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
