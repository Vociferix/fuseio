use super::Req;
use crate::proto::request::Bmap;
use crate::types::Ino;

#[derive(Debug)]
pub struct MapBlockReq<C> {
    req: Req<C>,
    bmap: Bmap,
}

impl<C> MapBlockReq<C> {
    pub(crate) fn new(req: Req<C>, bmap: Bmap) -> Self {
        Self { req, bmap }
    }

    pub fn req(&self) -> &Req<C> {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.bmap.ino()
    }

    pub fn block(&self) -> u64 {
        self.bmap.block()
    }

    pub fn block_size(&self) -> usize {
        self.bmap.block_size()
    }
}

impl<C> std::ops::Deref for MapBlockReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
