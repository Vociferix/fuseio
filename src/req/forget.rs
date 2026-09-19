use super::Req;
use crate::proto::request::{BatchForget, Forget};
use crate::types::ForgetIno;

pub struct ForgetReq {
    req: Req,
    inner: Inner,
}

enum Inner {
    Single(ForgetIno),
    Batch(BatchForget),
}

impl ForgetReq {
    pub(crate) fn from_single(req: Req, forget: Forget) -> Self {
        Self {
            req,
            inner: Inner::Single(ForgetIno::new(forget.ino(), forget.nlookup())),
        }
    }

    pub(crate) fn from_batch(req: Req, batch: BatchForget) -> Self {
        Self {
            req,
            inner: Inner::Batch(batch),
        }
    }

    pub fn inos(&self) -> &[ForgetIno] {
        match &self.inner {
            Inner::Single(single) => unsafe { std::slice::from_raw_parts(single, 1) },
            Inner::Batch(batch) => batch.inos(),
        }
    }
}

impl std::ops::Deref for ForgetReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}

impl std::fmt::Debug for ForgetReq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ForgetReq")
            .field("req", &self.req)
            .field("inos", &self.inos())
            .finish()
    }
}
