use crate::Result;
use crate::buf::{BufPool, IntoIoBuf};
use crate::dev_fuse::FuseChannel;
use crate::fs::types::PassthroughFd;
use crate::proto::notify::{
    Delete, EncodeNotify, ExpireEntry, IncrementEpoch, InvalEntry, InvalInode, Retrieve, Store,
};
use crate::proto::request::{Cfg, NotifyReply};
use crate::server::ServerInner;
use crate::types::{FileRange, Ino, Version};

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::rc::Rc;

#[derive(Clone)]
pub struct Context {
    server: Rc<ServerInner>,
}

#[derive(Debug, Clone)]
pub struct CacheData {
    reply: NotifyReply,
}

impl Context {
    pub(crate) fn new(server: Rc<ServerInner>) -> Self {
        Self { server }
    }

    pub fn proto_version(&self) -> Version {
        Version(crate::handshake::MAJOR_VER, self.server.minor_ver)
    }

    pub fn open_passthrough<T>(&self, fd: T) -> Result<PassthroughFd<T>>
    where
        T: std::os::fd::AsFd,
    {
        PassthroughFd::open(fd, &self.server.dev)
    }

    pub fn buffer_pool(&self) -> &BufPool {
        &self.server.buf_pool
    }

    pub(crate) fn cfg(&self) -> Cfg {
        Cfg {
            minor_ver: self.server.minor_ver,
            flags: self.server.flags,
        }
    }

    pub(crate) fn dev(&self) -> &FuseChannel {
        &self.server.dev
    }

    pub async fn invalidate_inode(&self, ino: Ino) -> Result<()> {
        self.server
            .dev
            .write_buf(InvalInode::new(ino).encode(self.cfg())?)
            .await
            .0?;
        Ok(())
    }

    pub async fn invalidate_inode_range<R>(&self, ino: Ino, range: R) -> Result<()>
    where
        R: Into<FileRange>,
    {
        self.server
            .dev
            .write_buf(InvalInode::new(ino).range(range).encode(self.cfg())?)
            .await
            .0?;
        Ok(())
    }

    pub async fn delete_inode<N>(&self, parent: Ino, child: Ino, name: N) -> Result<()>
    where
        N: AsRef<OsStr>,
    {
        let name = name.as_ref().as_bytes();
        let mut name_buf = self.buffer_pool().checkout_with_capacity(name.len());
        name_buf.extend_from_slice(name);

        self.server
            .dev
            .write_buf(Delete::new(parent, child, name_buf).encode(self.cfg())?)
            .await
            .0?;
        Ok(())
    }

    pub async fn invalidate_entry<N>(&self, parent: Ino, name: N) -> Result<()>
    where
        N: AsRef<OsStr>,
    {
        let name = name.as_ref().as_bytes();
        let mut name_buf = self.buffer_pool().checkout_with_capacity(name.len());
        name_buf.extend_from_slice(name);

        self.server
            .dev
            .write_buf(InvalEntry::new(parent, name_buf).encode(self.cfg())?)
            .await
            .0?;
        Ok(())
    }

    // TODO: kernels without EXPIRE_ONLY (before 7.38) ignore the flag and fully
    // invalidate; libfuse returns ENOSYS instead. None of the notifications
    // check the minimum protocol version (inval: 7.12, store/retrieve: 7.15,
    // delete: 7.18). Also worth documenting: on Linux, inval_entry/delete lock
    // the parent directory, so sending them from a handler for an operation on
    // that directory deadlocks.
    pub async fn expire_entry<N>(&self, parent: Ino, name: N) -> Result<()>
    where
        N: AsRef<OsStr>,
    {
        let name = name.as_ref().as_bytes();
        let mut name_buf = self.buffer_pool().checkout_with_capacity(name.len());
        name_buf.extend_from_slice(name);

        self.server
            .dev
            .write_buf(ExpireEntry::new(parent, name_buf).encode(self.cfg())?)
            .await
            .0?;
        Ok(())
    }

    pub async fn increment_epoch(&self) -> Result<()> {
        self.server
            .dev
            .write_buf(IncrementEpoch::new().encode(self.cfg())?)
            .await
            .0?;
        Ok(())
    }

    pub async fn set_cache<B>(&self, ino: Ino, offset: u64, data: B) -> Result<()>
    where
        B: IntoIoBuf,
    {
        self.server
            .dev
            .write_buf(Store::new(ino, offset, data).encode(self.cfg())?)
            .await
            .0?;
        Ok(())
    }

    pub async fn get_cache(&self, ino: Ino, offset: u64, len: usize) -> Result<CacheData> {
        struct Guard<'a> {
            replies: &'a crate::server::ReplyState,
            id: u32,
        }

        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                self.replies.pending.borrow_mut().remove(&self.id);
            }
        }

        let (id, rx) = self.server.replies.channel();

        let guard = Guard {
            replies: &self.server.replies,
            id,
        };

        let id = ((self.server.id as u64) << 32) | (id as u64);

        self.server
            .dev
            .write_buf(Retrieve::new(id, ino, offset, len).encode(self.cfg())?)
            .await
            .0?;

        let Some(reply) = rx.await else {
            return Err(crate::Error::EIO);
        };

        std::mem::forget(guard);

        Ok(CacheData { reply })
    }
}

impl std::fmt::Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Context").finish_non_exhaustive()
    }
}

impl CacheData {
    pub fn offset(&self) -> u64 {
        self.reply.offset()
    }

    pub fn data(&self) -> &[u8] {
        self.reply.data()
    }
}
