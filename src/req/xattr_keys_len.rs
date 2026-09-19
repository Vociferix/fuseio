use super::Req;
use crate::proto::request::ListXattr;
use crate::types::Ino;

#[derive(Debug)]
pub struct XattrKeysLenReq {
    req: Req,
    listxattr: ListXattr,
}

impl XattrKeysLenReq {
    pub(crate) fn new(req: Req, listxattr: ListXattr) -> Self {
        Self { req, listxattr }
    }

    pub fn req(&self) -> &Req {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.listxattr.ino()
    }
}

impl std::ops::Deref for XattrKeysLenReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
