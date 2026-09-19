use super::Req;
use crate::proto::request::Release;
use crate::types::{FileHandle, Ino, LockOwner, OFlag};

#[derive(Debug)]
pub struct CloseReq<'a> {
    req: Req<'a>,
    release: Release,
}

impl<'a> CloseReq<'a> {
    pub(crate) fn new(req: Req<'a>, release: Release) -> Self {
        Self { req, release }
    }

    pub fn ino(&self) -> Ino {
        self.release.ino()
    }

    pub fn file_handle(&self) -> FileHandle {
        self.release.file_handle()
    }

    pub fn flags(&self) -> OFlag {
        self.release.flags()
    }

    pub fn flush(&self) -> bool {
        self.release.flush()
    }

    pub fn flock_unlock(&self) -> bool {
        self.release.flock_unlock()
    }

    pub fn lock_owner(&self) -> Option<LockOwner> {
        self.release.lock_owner()
    }
}

impl<'a> std::ops::Deref for CloseReq<'a> {
    type Target = Req<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
