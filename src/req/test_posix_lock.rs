use super::Req;
use crate::proto::request::GetLk;
use crate::types::{FileHandle, FileRange, Ino, LockKind, LockOwner, Pid};

#[derive(Debug)]
pub struct TestPosixLockReq {
    req: Req,
    getlk: GetLk,
}

impl TestPosixLockReq {
    pub(crate) fn new(req: Req, getlk: GetLk) -> Self {
        Self { req, getlk }
    }

    pub fn req(&self) -> &Req {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.getlk.ino()
    }

    pub fn file_handle(&self) -> FileHandle {
        self.getlk.file_handle()
    }

    pub fn lock_owner(&self) -> Option<LockOwner> {
        self.getlk.lock_owner()
    }

    pub fn file_range(&self) -> FileRange {
        self.getlk.file_range()
    }

    pub fn lock_kind(&self) -> LockKind {
        self.getlk.lock_kind()
    }

    pub fn pid(&self) -> Pid {
        unsafe { self.getlk.pid().unwrap_unchecked() }
    }
}

impl std::ops::Deref for TestPosixLockReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
