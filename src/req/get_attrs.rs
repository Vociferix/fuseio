use super::Req;
use crate::proto::request::GetAttr;
use crate::types::{FileHandle, Ino};

#[derive(Debug)]
pub struct GetAttrsReq {
    req: Req,
    get_attr: GetAttr,
}

impl GetAttrsReq {
    pub(crate) fn new(req: Req, get_attr: GetAttr) -> Self {
        Self { req, get_attr }
    }

    pub fn req(&self) -> &Req {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.get_attr.ino()
    }

    pub fn file_handle(&self) -> Option<FileHandle> {
        self.get_attr.file_handle()
    }
}

impl std::ops::Deref for GetAttrsReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
