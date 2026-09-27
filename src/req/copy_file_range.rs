use super::Req;
use crate::proto::request::CopyFileRange;
use crate::types::CopyFileRangePos;

#[derive(Debug)]
pub struct CopyFileRangeReq<C> {
    req: Req<C>,
    copy_file_range: CopyFileRange,
}

impl<C> CopyFileRangeReq<C> {
    pub(crate) fn new(req: Req<C>, copy_file_range: CopyFileRange) -> Self {
        Self {
            req,
            copy_file_range,
        }
    }

    pub fn req(&self) -> &Req<C> {
        &self.req
    }

    pub fn src(&self) -> &CopyFileRangePos {
        self.copy_file_range.src()
    }

    pub fn dst(&self) -> &CopyFileRangePos {
        self.copy_file_range.dst()
    }

    pub fn len(&self) -> u64 {
        self.copy_file_range.len()
    }
}

impl<C> std::ops::Deref for CopyFileRangeReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
