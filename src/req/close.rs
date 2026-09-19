use super::Req;
use crate::proto::request::Release;
use crate::types::{FileHandle, Ino, LockOwner, OFlag};

#[derive(Debug)]
pub struct CloseReq {
    req: Req,
    release: Release,
}

impl CloseReq {
    pub(crate) fn new(req: Req, release: Release) -> Self {
        Self { req, release }
    }

    pub fn req(&self) -> &Req {
        &self.req
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

impl std::ops::Deref for CloseReq {
    type Target = Req;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
