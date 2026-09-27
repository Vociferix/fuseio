use super::Req;
use crate::proto::request::MkNod;
use crate::types::{DeviceNumber, Ino, InodeKind, Mode};

use std::ffi::OsStr;

#[derive(Debug)]
pub struct MakeNodeReq<C> {
    req: Req<C>,
    mknod: MkNod,
}

impl<C> MakeNodeReq<C> {
    pub(crate) fn new(req: Req<C>, mknod: MkNod) -> Self {
        Self { req, mknod }
    }

    pub fn req(&self) -> &Req<C> {
        &self.req
    }

    pub fn parent(&self) -> Ino {
        self.mknod.parent()
    }

    pub fn mode(&self) -> Mode {
        self.mknod.mode()
    }

    pub fn umask(&self) -> Mode {
        self.mknod.umask()
    }

    pub fn kind(&self) -> InodeKind {
        self.mknod.kind()
    }

    pub fn device_number(&self) -> Option<DeviceNumber> {
        self.mknod.device_number()
    }

    pub fn name(&self) -> &OsStr {
        self.mknod.name()
    }
}

impl<C> std::ops::Deref for MakeNodeReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
