use super::Req;
use crate::proto::request::Lookup;
use crate::types::Ino;

use std::ffi::OsStr;

#[derive(Debug)]
pub struct LookupReq<C> {
    req: Req<C>,
    lookup: Lookup,
}

impl<C> LookupReq<C> {
    pub(crate) fn new(req: Req<C>, lookup: Lookup) -> Self {
        Self { req, lookup }
    }

    pub fn req(&self) -> &Req<C> {
        &self.req
    }

    pub fn parent(&self) -> Ino {
        self.lookup.parent()
    }

    pub fn name(&self) -> &OsStr {
        self.lookup.name()
    }
}

impl<C> std::ops::Deref for LookupReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
