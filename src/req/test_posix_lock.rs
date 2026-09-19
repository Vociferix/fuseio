use super::Req;
use crate::proto::request::GetLk;
use crate::types::{FileHandle, FileRange, Ino, LockKind, LockOwner, Pid};

#[derive(Debug)]
pub struct TestPosixLockReq<'a> {
    req: Req<'a>,
    getlk: GetLk,
}

impl<'a> TestPosixLockReq<'a> {
    pub(crate) fn new(req: Req<'a>, getlk: GetLk) -> Self {
        Self { req, getlk }
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

impl<'a> std::ops::Deref for TestPosixLockReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
