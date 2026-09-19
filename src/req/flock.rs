use super::Req;
use crate::proto::request::SetLk;
use crate::types::{FileHandle, Ino, LockKind};

#[derive(Debug)]
pub struct FlockReq<'a> {
    req: Req<'a>,
    setlk: SetLk,
}

impl<'a> FlockReq<'a> {
    pub(crate) fn new(req: Req<'a>, setlk: SetLk) -> Self {
        Self { req, setlk }
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

impl<'a> std::ops::Deref for FlockReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
