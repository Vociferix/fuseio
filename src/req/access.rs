use super::Req;
use crate::proto::request::Access;
use crate::types::{AccessFlags, Ino};

#[derive(Debug)]
pub struct AccessReq {
    req: Req,
    access: Access,
}

impl AccessReq {
    pub(crate) fn new(req: Req, access: Access) -> Self {
        Self { req, access }
    }

    pub fn ino(&self) -> Ino {
        self.access.ino()
    }

    pub fn flags(&self) -> AccessFlags {
        self.access.flags()
    }
}

impl std::ops::Deref for AccessReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
