use super::Req;
use crate::proto::request::Lookup;
use crate::types::Ino;

use std::ffi::OsStr;

#[derive(Debug)]
pub struct LookupReq<'a> {
    req: Req<'a>,
    lookup: Lookup,
}

impl<'a> LookupReq<'a> {
    pub(crate) fn new(req: Req<'a>, lookup: Lookup) -> Self {
        Self { req, lookup }
    }

    pub fn parent(&self) -> Ino {
        self.lookup.parent()
    }

    pub fn name(&self) -> &OsStr {
        self.lookup.name()
    }
}

impl<'a> std::ops::Deref for LookupReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
