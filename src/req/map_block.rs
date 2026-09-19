use super::Req;
use crate::proto::request::Bmap;
use crate::types::Ino;

#[derive(Debug)]
pub struct MapBlockReq {
    req: Req,
    bmap: Bmap,
}

impl MapBlockReq {
    pub(crate) fn new(req: Req, bmap: Bmap) -> Self {
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

impl std::ops::Deref for MapBlockReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
