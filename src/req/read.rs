use super::Req;
use crate::proto::request::Read;
use crate::types::{FileHandle, Ino, LockOwner, OFlag};

#[derive(Debug)]
pub struct ReadReq<C> {
    req: Req<C>,
    read: Read,
}

impl<C> ReadReq<C> {
    pub(crate) fn new(req: Req<C>, read: Read) -> Self {
        Self { req, read }
    }

    pub fn req(&self) -> &Req<C> {
        &self.req
    }

    pub fn ino(&self) -> Ino {
        self.read.ino()
    }

    pub fn file_handle(&self) -> FileHandle {
        self.read.file_handle()
    }

    pub fn offset(&self) -> u64 {
        self.read.offset()
    }

    // Not a collection: this is a byte count the kernel asked for, and a zero
    // one is meaningful rather than "empty".
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.read.len()
    }

    pub fn lock_owner(&self) -> Option<LockOwner> {
        self.read.lock_owner()
    }

    pub fn open_flags(&self) -> OFlag {
        self.read.open_flags()
    }
}

impl<C> std::ops::Deref for ReadReq<C> {
    type Target = Req<C>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
