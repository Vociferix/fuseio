use super::Req;
use crate::proto::request::Bmap;
use crate::types::Ino;

#[derive(Debug)]
pub struct MapBlockReq<'a> {
    req: Req<'a>,
    bmap: Bmap,
}

impl<'a> MapBlockReq<'a> {
    pub(crate) fn new(req: Req<'a>, bmap: Bmap) -> Self {
        Self { req, bmap }
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

impl<'a> std::ops::Deref for MapBlockReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
