use super::Req;
use crate::proto::request::ReadLink;
use crate::types::Ino;

#[derive(Debug)]
pub struct ReadLinkReq<'a> {
    req: Req<'a>,
    read_link: ReadLink,
}

impl<'a> ReadLinkReq<'a> {
    pub(crate) fn new(req: Req<'a>, read_link: ReadLink) -> Self {
        Self { req, read_link }
    }

    pub fn ino(&self) -> Ino {
        self.read_link.ino()
    }
}

impl<'a> std::ops::Deref for ReadLinkReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
