use super::Req;
use crate::proto::request::ListXattr;
use crate::types::Ino;

#[derive(Debug)]
pub struct XattrKeysReq<'a> {
    req: Req<'a>,
    listxattr: ListXattr,
}

impl<'a> XattrKeysReq<'a> {
    pub(crate) fn new(req: Req<'a>, listxattr: ListXattr) -> Self {
        Self { req, listxattr }
    }

    pub fn ino(&self) -> Ino {
        self.listxattr.ino()
    }

    pub fn len(&self) -> usize {
        self.listxattr.len()
    }
}

impl<'a> std::ops::Deref for XattrKeysReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
