use crate::Result;
use crate::buf::{BufPool, IntoIoBuf};
use crate::dev_fuse::FuseChannel;
use crate::passthrough::PassthroughFd;
use crate::proto::Cfg;
use crate::proto::notify::{
    Delete, EncodeNotify, ExpireEntry, IncrementEpoch, InvalEntry, InvalInode, Retrieve, Store,
};
use crate::proto::request::NotifyReply;
use crate::server::{ReplyState, ServerInner};
use crate::types::{FileRange, Gid, Ino, Pid, Request, Uid, Version};

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::rc::Rc;

mod access;
mod close;
mod copy_file_range;
mod create_file;
mod fallocate;
mod flock;
mod flush;
mod forget;
mod fsync;
mod get_attrs;
mod get_xattr;
mod get_xattr_len;
mod ioctl;
mod link;
mod lookup;
mod lseek;
mod make_dir;
mod make_node;
mod map_block;
mod open;
pub(crate) mod poll;
mod posix_lock;
mod read;
mod read_dir;
mod read_link;
mod remove_dir;
mod remove_xattr;
mod rename;
mod set_attrs;
mod set_xattr;
mod statfs;
mod statx;
mod symlink;
mod syncfs;
mod test_posix_lock;
mod tmp_file;
mod unlink_node;
mod write;
mod xattr_keys;
mod xattr_keys_len;

pub use access::AccessReq;
pub use close::CloseReq;
pub use copy_file_range::CopyFileRangeReq;
pub use create_file::CreateFileReq;
pub use fallocate::FallocateReq;
pub use flock::FlockReq;
pub use flush::FlushReq;
pub use forget::ForgetReq;
pub use fsync::FsyncReq;
pub use get_attrs::GetAttrsReq;
pub use get_xattr::GetXattrReq;
pub use get_xattr_len::GetXattrLenReq;
pub use ioctl::IoctlReq;
pub use link::LinkReq;
pub use lookup::LookupReq;
pub use lseek::LseekReq;
pub use make_dir::MakeDirReq;
pub use make_node::MakeNodeReq;
pub use map_block::MapBlockReq;
pub use open::OpenReq;
pub use poll::PollReq;
pub use posix_lock::PosixLockReq;
pub use read::ReadReq;
pub use read_dir::ReadDirReq;
pub use read_link::ReadLinkReq;
pub use remove_dir::RemoveDirReq;
pub use remove_xattr::RemoveXattrReq;
pub use rename::RenameReq;
pub use set_attrs::SetAttrsReq;
pub use set_xattr::SetXattrReq;
pub use statfs::StatFsReq;
pub use statx::StatXReq;
pub use symlink::SymlinkReq;
pub use syncfs::SyncFsReq;
pub use test_posix_lock::TestPosixLockReq;
pub use tmp_file::TmpFileReq;
pub use unlink_node::UnlinkNodeReq;
pub use write::WriteReq;
pub use xattr_keys::XattrKeysReq;
pub use xattr_keys_len::XattrKeysLenReq;

#[derive(Clone, Copy)]
pub struct Req<'a> {
    server: &'a ServerInner,
    req: Request,
}

#[derive(Clone)]
pub struct Notify {
    id: usize,
    dev: Rc<FuseChannel>,
    cfg: Cfg,
    pool: BufPool,
    replies: Rc<ReplyState>,
}

#[derive(Debug, Clone)]
pub struct CacheData {
    reply: NotifyReply,
}

trait NotifyOps {
    fn id(&self) -> usize;

    fn dev(&self) -> &FuseChannel;

    fn cfg(&self) -> Cfg;

    fn pool(&self) -> &BufPool;

    fn replies(&self) -> &ReplyState;

    async fn invalidate_inode(&self, ino: Ino) -> Result<()> {
        self.dev()
            .write_buf(InvalInode::new(ino).encode(self.cfg())?)
            .await
            .0?;
        Ok(())
    }

    async fn invalidate_inode_range<R>(&self, ino: Ino, range: R) -> Result<()>
    where
        R: Into<FileRange>,
    {
        self.dev()
            .write_buf(InvalInode::new(ino).range(range).encode(self.cfg())?)
            .await
            .0?;
        Ok(())
    }

    async fn delete_inode<N>(&self, parent: Ino, child: Ino, name: N) -> Result<()>
    where
        N: AsRef<OsStr>,
    {
        let name = name.as_ref().as_bytes();
        let mut name_buf = self.pool().checkout_with_capacity(name.len());
        name_buf.extend_from_slice(name);

        self.dev()
            .write_buf(Delete::new(parent, child, name_buf).encode(self.cfg())?)
            .await
            .0?;
        Ok(())
    }

    async fn invalidate_entry<N>(&self, parent: Ino, name: N) -> Result<()>
    where
        N: AsRef<OsStr>,
    {
        let name = name.as_ref().as_bytes();
        let mut name_buf = self.pool().checkout_with_capacity(name.len());
        name_buf.extend_from_slice(name);

        self.dev()
            .write_buf(InvalEntry::new(parent, name_buf).encode(self.cfg())?)
            .await
            .0?;
        Ok(())
    }

    async fn expire_entry<N>(&self, parent: Ino, name: N) -> Result<()>
    where
        N: AsRef<OsStr>,
    {
        let name = name.as_ref().as_bytes();
        let mut name_buf = self.pool().checkout_with_capacity(name.len());
        name_buf.extend_from_slice(name);

        self.dev()
            .write_buf(ExpireEntry::new(parent, name_buf).encode(self.cfg())?)
            .await
            .0?;
        Ok(())
    }

    async fn increment_epoch(&self) -> Result<()> {
        self.dev()
            .write_buf(IncrementEpoch::new().encode(self.cfg())?)
            .await
            .0?;
        Ok(())
    }

    async fn set_cache<B>(&self, ino: Ino, offset: u64, data: B) -> Result<()>
    where
        B: IntoIoBuf,
    {
        self.dev()
            .write_buf(Store::new(ino, offset, data).encode(self.cfg())?)
            .await
            .0?;
        Ok(())
    }

    async fn get_cache(&self, ino: Ino, offset: u64, len: usize) -> Result<CacheData> {
        let (id, rx) = self.replies().channel();

        let id = ((self.id() as u64) << 32) | (id as u64);

        let res = self
            .dev()
            .write_buf(Retrieve::new(id, ino, offset, len).encode(self.cfg())?)
            .await
            .0;

        if let Err(err) = res {
            self.replies().pending.borrow_mut().remove(&(id as u32));
            return Err(err.into());
        }

        let Some(reply) = rx.await else {
            return Err(crate::Error::EIO);
        };

        Ok(CacheData::new(reply))
    }
}

impl<'a> Req<'a> {
    pub(crate) fn new<F, U>(server: &'a crate::server::Server<F, U>, req: Request) -> Self {
        Self { server, req }
    }

    pub fn open_passthrough<T>(&self, fd: T) -> Result<PassthroughFd<T>>
    where
        T: std::os::fd::AsFd,
    {
        PassthroughFd::open(fd, &self.server.dev)
    }

    pub fn buffer_pool(&self) -> &'a BufPool {
        &self.server.buf_pool
    }

    pub fn version(&self) -> Version {
        Version(crate::handshake::MAJOR_VER, self.server.minor_ver)
    }

    pub fn id(&self) -> u64 {
        self.req.id()
    }

    pub fn uid(&self) -> Uid {
        self.req.uid()
    }

    pub fn gid(&self) -> Gid {
        self.req.gid()
    }

    pub fn pid(&self) -> Pid {
        self.req.pid()
    }

    pub fn notify_handle(&self) -> Notify {
        Notify {
            id: self.server.id,
            dev: self.server.dev.clone(),
            cfg: self.cfg(),
            pool: self.buffer_pool().clone(),
            replies: self.server.replies.clone(),
        }
    }

    pub async fn invalidate_inode(&self, ino: Ino) -> Result<()> {
        NotifyOps::invalidate_inode(self, ino).await
    }

    pub async fn invalidate_inode_range<R>(&self, ino: Ino, range: R) -> Result<()>
    where
        R: Into<FileRange>,
    {
        NotifyOps::invalidate_inode_range(self, ino, range).await
    }

    pub async fn delete_inode<N>(&self, parent: Ino, child: Ino, name: N) -> Result<()>
    where
        N: AsRef<OsStr>,
    {
        NotifyOps::delete_inode(self, parent, child, name).await
    }

    pub async fn invalidate_entry<N>(&self, parent: Ino, name: N) -> Result<()>
    where
        N: AsRef<OsStr>,
    {
        NotifyOps::invalidate_entry(self, parent, name).await
    }

    pub async fn expire_entry<N>(&self, parent: Ino, name: N) -> Result<()>
    where
        N: AsRef<OsStr>,
    {
        NotifyOps::expire_entry(self, parent, name).await
    }

    pub async fn increment_epoch(&self) -> Result<()> {
        NotifyOps::increment_epoch(self).await
    }

    pub async fn set_cache<B>(&self, ino: Ino, offset: u64, data: B) -> Result<()>
    where
        B: IntoIoBuf,
    {
        NotifyOps::set_cache(self, ino, offset, data).await
    }

    pub async fn get_cache(&self, ino: Ino, offset: u64, len: usize) -> Result<CacheData> {
        NotifyOps::get_cache(self, ino, offset, len).await
    }
}

impl std::fmt::Debug for Req<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.req, f)
    }
}

impl NotifyOps for Req<'_> {
    fn id(&self) -> usize {
        self.server.id
    }

    fn dev(&self) -> &FuseChannel {
        &self.server.dev
    }

    fn cfg(&self) -> Cfg {
        Cfg {
            minor_ver: self.server.minor_ver,
            flags: self.server.flags,
        }
    }

    fn pool(&self) -> &BufPool {
        &self.server.buf_pool
    }

    fn replies(&self) -> &ReplyState {
        &self.server.replies
    }
}

impl Notify {
    pub async fn invalidate_inode(&self, ino: Ino) -> Result<()> {
        NotifyOps::invalidate_inode(self, ino).await
    }

    pub async fn invalidate_inode_range<R>(&self, ino: Ino, range: R) -> Result<()>
    where
        R: Into<FileRange>,
    {
        NotifyOps::invalidate_inode_range(self, ino, range).await
    }

    pub async fn delete_inode<N>(&self, parent: Ino, child: Ino, name: N) -> Result<()>
    where
        N: AsRef<OsStr>,
    {
        NotifyOps::delete_inode(self, parent, child, name).await
    }

    pub async fn invalidate_entry<N>(&self, parent: Ino, name: N) -> Result<()>
    where
        N: AsRef<OsStr>,
    {
        NotifyOps::invalidate_entry(self, parent, name).await
    }

    pub async fn expire_entry<N>(&self, parent: Ino, name: N) -> Result<()>
    where
        N: AsRef<OsStr>,
    {
        NotifyOps::expire_entry(self, parent, name).await
    }

    pub async fn increment_epoch(&self) -> Result<()> {
        NotifyOps::increment_epoch(self).await
    }

    pub async fn set_cache<B>(&self, ino: Ino, offset: u64, data: B) -> Result<()>
    where
        B: IntoIoBuf,
    {
        NotifyOps::set_cache(self, ino, offset, data).await
    }

    pub async fn get_cache(&self, ino: Ino, offset: u64, len: usize) -> Result<CacheData> {
        NotifyOps::get_cache(self, ino, offset, len).await
    }
}

impl std::fmt::Debug for Notify {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Notify").finish()
    }
}

impl NotifyOps for Notify {
    fn id(&self) -> usize {
        self.id
    }

    fn dev(&self) -> &FuseChannel {
        &self.dev
    }

    fn cfg(&self) -> Cfg {
        self.cfg
    }

    fn pool(&self) -> &BufPool {
        &self.pool
    }

    fn replies(&self) -> &ReplyState {
        &self.replies
    }
}

impl CacheData {
    pub(crate) fn new(reply: NotifyReply) -> Self {
        Self { reply }
    }

    pub fn offset(&self) -> u64 {
        self.reply.offset()
    }

    pub fn data(&self) -> &[u8] {
        self.reply.data()
    }
}
