use super::Req;
use crate::proto::request::Link;
use crate::types::Ino;

use std::ffi::OsStr;

#[derive(Debug)]
pub struct LinkReq<C> {
    req: Req<C>,
    link: Link,
}

impl<C> LinkReq<C> {
    pub(crate) fn new(req: Req<C>, link: Link) -> Self {
        Self { req, link }
    }

    pub fn req(&self) -> &Req<C> {
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

impl<C> std::ops::Deref for LinkReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
