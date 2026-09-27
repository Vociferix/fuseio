use super::Req;
use crate::proto::request::ReadLink;
use crate::types::Ino;

#[derive(Debug)]
pub struct ReadLinkReq<C> {
    req: Req<C>,
    read_link: ReadLink,
}

impl<C> ReadLinkReq<C> {
    pub(crate) fn new(req: Req<C>, read_link: ReadLink) -> Self {
        Self { req, read_link }
    }

    pub fn req(&self) -> &Req<C> {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.read_link.ino()
    }
}

impl<C> std::ops::Deref for ReadLinkReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
