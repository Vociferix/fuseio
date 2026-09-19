use super::Req;
use crate::proto::request::ListXattr;
use crate::types::Ino;

#[derive(Debug)]
pub struct XattrKeysLenReq<'a> {
    req: Req<'a>,
    listxattr: ListXattr,
}

impl<'a> XattrKeysLenReq<'a> {
    pub(crate) fn new(req: Req<'a>, listxattr: ListXattr) -> Self {
        Self { req, listxattr }
    }

    pub fn ino(&self) -> Ino {
        self.listxattr.ino()
    }
}

impl<'a> std::ops::Deref for XattrKeysLenReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
