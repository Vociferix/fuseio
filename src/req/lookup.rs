use super::Req;
use crate::proto::request::Lookup;
use crate::types::Ino;

use std::ffi::OsStr;

#[derive(Debug)]
pub struct LookupReq {
    req: Req,
    lookup: Lookup,
}

impl LookupReq {
    pub(crate) fn new(req: Req, lookup: Lookup) -> Self {
        Self { req, lookup }
    }

    pub fn req(&self) -> &Req {
        &self.req
    }

    pub fn parent(&self) -> Ino {
        self.lookup.parent()
    }

    pub fn name(&self) -> &OsStr {
        self.lookup.name()
    }
}

impl std::ops::Deref for LookupReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
