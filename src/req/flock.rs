use super::Req;
use crate::proto::request::SetLk;
use crate::types::{FileHandle, Ino, LockKind, LockOwner, Tgid};

#[derive(Debug)]
pub struct FlockReq<C> {
    req: Req<C>,
    setlk: SetLk,
}

impl<C> FlockReq<C> {
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

    pub fn lock_kind(&self) -> LockKind {
        self.setlk.lock_kind()
    }

    /// The open file the lock belongs to, which a later close reports when it
    /// releases the lock.
    pub fn lock_owner(&self) -> Option<LockOwner> {
        self.setlk.lock_owner()
    }

    /// The thread group that asked for the lock, if any: a kernel sends none
    /// when releasing one.
    pub fn tgid(&self) -> Option<Tgid> {
        self.setlk.tgid()
    }
}

impl<C> std::ops::Deref for FlockReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
