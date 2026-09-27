use super::Req;
use crate::proto::request::SetLk;
use crate::types::{FileHandle, FileRange, Ino, LockKind, LockOwner, Tgid};

#[derive(Debug)]
pub struct PosixLockReq<C> {
    req: Req<C>,
    setlk: SetLk,
}

impl<C> PosixLockReq<C> {
    pub(crate) fn new(req: Req<C>, setlk: SetLk) -> Self {
        Self { req, setlk }
    }

    pub fn req(&self) -> &Req<C> {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.setlk.ino()
    }

    pub fn file_handle(&self) -> FileHandle {
        self.setlk.file_handle()
    }

    pub fn lock_owner(&self) -> Option<LockOwner> {
        self.setlk.lock_owner()
    }

    pub fn file_range(&self) -> FileRange {
        self.setlk.file_range()
    }

    pub fn lock_kind(&self) -> LockKind {
        self.setlk.lock_kind()
    }

    /// The thread group that asked for the lock, if any: a kernel sends none
    /// when releasing one.
    pub fn tgid(&self) -> Option<Tgid> {
        self.setlk.tgid()
    }
}

impl<C> std::ops::Deref for PosixLockReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
