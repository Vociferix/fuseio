use super::Req;
use crate::proto::request::ReadLink;
use crate::types::Ino;

#[derive(Debug)]
pub struct ReadLinkReq {
    req: Req,
    read_link: ReadLink,
}

impl ReadLinkReq {
    pub(crate) fn new(req: Req, read_link: ReadLink) -> Self {
        Self { req, read_link }
    }

    pub fn ino(&self) -> Ino {
        self.read_link.ino()
    }
}

impl std::ops::Deref for ReadLinkReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
