use super::Req;
use crate::proto::request::Access;
use crate::types::{AccessFlags, Ino};

#[derive(Debug)]
pub struct AccessReq<'a> {
    req: Req<'a>,
    access: Access,
}

impl<'a> AccessReq<'a> {
    pub(crate) fn new(req: Req<'a>, access: Access) -> Self {
        Self { req, access }
    }

    pub fn ino(&self) -> Ino {
        self.access.ino()
    }

    pub fn flags(&self) -> AccessFlags {
        self.access.flags()
    }
}

impl<'a> std::ops::Deref for AccessReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
