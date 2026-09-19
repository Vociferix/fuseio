use super::Req;
use crate::proto::request::Link;
use crate::types::Ino;

use std::ffi::OsStr;

#[derive(Debug)]
pub struct LinkReq<'a> {
    req: Req<'a>,
    link: Link,
}

impl<'a> LinkReq<'a> {
    pub(crate) fn new(req: Req<'a>, link: Link) -> Self {
        Self { req, link }
    }

    pub fn dst(&self) -> Ino {
        self.link.dst()
    }

    pub fn src(&self) -> Ino {
        self.link.src()
    }

    pub fn name(&self) -> &OsStr {
        self.link.name()
    }
}

impl<'a> std::ops::Deref for LinkReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
