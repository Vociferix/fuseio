use super::Req;
use crate::proto::request::GetAttr;
use crate::types::{FileHandle, Ino};

#[derive(Debug)]
pub struct GetAttrsReq<C> {
    req: Req<C>,
    get_attr: GetAttr,
}

impl<C> GetAttrsReq<C> {
    pub(crate) fn new(req: Req<C>, get_attr: GetAttr) -> Self {
        Self { req, get_attr }
    }

    pub fn req(&self) -> &Req<C> {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.get_attr.ino()
    }

    pub fn file_handle(&self) -> Option<FileHandle> {
        self.get_attr.file_handle()
    }
}

impl<C> std::ops::Deref for GetAttrsReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
