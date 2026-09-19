use super::Req;
use crate::proto::request::SetLk;
use crate::types::{FileHandle, FileRange, Ino, LockKind, LockOwner, Pid};

#[derive(Debug)]
pub struct PosixLockReq<'a> {
    req: Req<'a>,
    setlk: SetLk,
}

impl<'a> PosixLockReq<'a> {
    pub(crate) fn new(req: Req<'a>, setlk: SetLk) -> Self {
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

impl<'a> std::ops::Deref for PosixLockReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
