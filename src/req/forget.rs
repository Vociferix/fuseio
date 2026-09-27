use super::Req;
use crate::proto::request::{BatchForget, Forget};
use crate::types::ForgetIno;

pub struct ForgetReq<C> {
    req: Req<C>,
    inner: Inner,
}

enum Inner {
    Single(ForgetIno),
    Batch(BatchForget),
}

impl<C> ForgetReq<C> {
    pub(crate) fn from_single(req: Req<C>, forget: Forget) -> Self {
        Self {
            req,
            inner: Inner::Single(ForgetIno::new(forget.ino(), forget.nlookup())),
        }
    }

    pub(crate) fn from_batch(req: Req<C>, batch: BatchForget) -> Self {
        Self {
            req,
            inner: Inner::Batch(batch),
        }
    }

    pub fn req(&self) -> &Req<C> {
        &self.req
    }

    pub fn inos(&self) -> &[ForgetIno] {
        match &self.inner {
            Inner::Single(single) => unsafe { std::slice::from_raw_parts(single, 1) },
            Inner::Batch(batch) => batch.inos(),
        }
    }
}

impl<C> std::ops::Deref for ForgetReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}

impl<C> std::fmt::Debug for ForgetReq<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ForgetReq")
            .field("req", &self.req)
            .field("inos", &self.inos())
            .finish()
    }
}
