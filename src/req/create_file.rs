use super::Req;
use crate::proto::request::Create;
use crate::types::{Ino, Mode, OFlag, OpenFlags};

use std::ffi::OsStr;

#[derive(Debug)]
pub struct CreateFileReq<'a> {
    req: Req<'a>,
    create: Create,
}

impl<'a> CreateFileReq<'a> {
    pub(crate) fn new(req: Req<'a>, create: Create) -> Self {
        Self { req, create }
    }

    pub fn parent(&self) -> Ino {
        self.create.parent()
    }

    pub fn mode(&self) -> Mode {
        self.create.mode()
    }

    pub fn umask(&self) -> Mode {
        self.create.umask()
    }

    pub fn flags(&self) -> OFlag {
        self.create.flags()
    }

    pub fn op_flags(&self) -> OpenFlags {
        self.create.op_flags()
    }

    pub fn name(&self) -> &OsStr {
        self.create.name()
    }
}

impl<'a> std::ops::Deref for CreateFileReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
