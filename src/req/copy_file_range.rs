use super::Req;
use crate::proto::request::CopyFileRange;
use crate::types::CopyFileRangePos;

#[derive(Debug)]
pub struct CopyFileRangeReq {
    req: Req,
    copy_file_range: CopyFileRange,
}

impl CopyFileRangeReq {
    pub(crate) fn new(req: Req, copy_file_range: CopyFileRange) -> Self {
        Self {
            req,
            copy_file_range,
        }
    }

    pub fn req(&self) -> &Req {
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

impl std::ops::Deref for CopyFileRangeReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
