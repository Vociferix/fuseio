use super::Req;
use crate::proto::request::SetLk;
use crate::types::{FileHandle, Ino, LockKind};

#[derive(Debug)]
pub struct FlockReq {
    req: Req,
    setlk: SetLk,
}

impl FlockReq {
    pub(crate) fn new(req: Req, setlk: SetLk) -> Self {
        Self { req, setlk }
    }

    pub fn req(&self) -> &Req {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.setlk.ino()
    }

    pub fn file_handle(&self) -> FileHandle {
        self.setlk.file_handle()
    }

    pub fn lock_kind(&self) -> LockKind {
        self.setlk.lock_kind()
    }
}

impl std::ops::Deref for FlockReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
