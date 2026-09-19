use super::Req;
use crate::proto::request::ListXattr;
use crate::types::Ino;

#[derive(Debug)]
pub struct XattrKeysReq {
    req: Req,
    listxattr: ListXattr,
}

impl XattrKeysReq {
    pub(crate) fn new(req: Req, listxattr: ListXattr) -> Self {
        Self { req, listxattr }
    }

    pub fn req(&self) -> &Req {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.listxattr.ino()
    }

    pub fn len(&self) -> usize {
        self.listxattr.len()
    }
}

impl std::ops::Deref for XattrKeysReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
