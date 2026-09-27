use super::Req;
use crate::proto::request::GetLk;
use crate::types::{FileHandle, FileRange, Ino, LockKind, LockOwner, Tgid};

#[derive(Debug)]
pub struct TestPosixLockReq<C> {
    req: Req<C>,
    getlk: GetLk,
}

impl<C> TestPosixLockReq<C> {
    pub(crate) fn new(req: Req<C>, getlk: GetLk) -> Self {
        Self { req, getlk }
    }

    pub fn req(&self) -> &Req<C> {
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

    /// The thread group that asked for the lock, if any: a kernel sends none
    /// when releasing one.
    pub fn tgid(&self) -> Option<Tgid> {
        self.getlk.tgid()
    }
}

impl<C> std::ops::Deref for TestPosixLockReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
