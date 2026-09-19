use super::Req;
use crate::proto::request::CopyFileRange;
use crate::types::CopyFileRangePos;

#[derive(Debug)]
pub struct CopyFileRangeReq<'a> {
    req: Req<'a>,
    copy_file_range: CopyFileRange,
}

impl<'a> CopyFileRangeReq<'a> {
    pub(crate) fn new(req: Req<'a>, copy_file_range: CopyFileRange) -> Self {
        Self {
            req,
            copy_file_range,
        }
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

impl<'a> std::ops::Deref for CopyFileRangeReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
