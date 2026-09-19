use super::Req;
use crate::proto::request::GetAttr;
use crate::types::{FileHandle, Ino};

#[derive(Debug)]
pub struct GetAttrsReq<'a> {
    req: Req<'a>,
    get_attr: GetAttr,
}

impl<'a> GetAttrsReq<'a> {
    pub(crate) fn new(req: Req<'a>, get_attr: GetAttr) -> Self {
        Self { req, get_attr }
    }

    pub fn ino(&self) -> Ino {
        self.get_attr.ino()
    }

    pub fn file_handle(&self) -> Option<FileHandle> {
        self.get_attr.file_handle()
    }
}

impl<'a> std::ops::Deref for GetAttrsReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
