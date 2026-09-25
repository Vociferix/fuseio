use super::Req;
use crate::proto::request::Link;
use crate::types::Ino;

use std::ffi::OsStr;

#[derive(Debug)]
pub struct LinkReq {
    req: Req,
    link: Link,
}

impl LinkReq {
    pub(crate) fn new(req: Req, link: Link) -> Self {
        Self { req, link }
    }

    pub fn req(&self) -> &Req {
        &self.req
    }

    pub fn parent(&self) -> Ino {
        self.link.parent()
    }

    pub fn ino(&self) -> Ino {
        self.link.ino()
    }

    pub fn name(&self) -> &OsStr {
        self.link.name()
    }
}

impl std::ops::Deref for LinkReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
