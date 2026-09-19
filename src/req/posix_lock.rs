use super::Req;
use crate::proto::request::SetLk;
use crate::types::{FileHandle, FileRange, Ino, LockKind, LockOwner, Pid};

#[derive(Debug)]
pub struct PosixLockReq {
    req: Req,
    setlk: SetLk,
}

impl PosixLockReq {
    pub(crate) fn new(req: Req, setlk: SetLk) -> Self {
        Self { req, setlk }
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

    pub fn pid(&self) -> Pid {
        unsafe { self.setlk.pid().unwrap_unchecked() }
    }
}

impl std::ops::Deref for PosixLockReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
