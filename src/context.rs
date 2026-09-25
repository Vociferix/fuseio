use crate::Result;
use crate::buf::{BufPool, IntoIoBuf};
use crate::dev_fuse::FuseChannel;
use crate::fs::types::{NotifyError, PassthroughFd};
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

    /// Hands a file to the kernel, so it serves reads and writes from it
    /// directly instead of sending them to this filesystem.
    ///
    /// Only Linux supports this, from 6.9, and only when built with
    /// `CONFIG_FUSE_PASSTHROUGH`. It also asks a lot of the caller:
    ///
    /// - [`FsCaps::PASSTHROUGH`](crate::types::FsCaps::PASSTHROUGH) has to be
    ///   enabled, which this checks;
    /// - the process needs `CAP_SYS_ADMIN`, or the kernel reports `EPERM`;
    /// - the file has to be a regular file, not a directory (`EISDIR`) or
    ///   anything else (`EINVAL`);
    /// - the file can't already be on a stack of filesystems as deep as the
    ///   kernel allows (`ELOOP`).
    ///
    /// Reports `ENOTSUP` where the platform or the connection has no passthrough
    /// at all.
    pub fn open_passthrough<T>(&self, fd: T) -> Result<PassthroughFd<T>>
    where
        T: std::os::fd::AsFd,
    {
        // The kernel reports EPERM whether the feature wasn't negotiated or the
        // process lacks CAP_SYS_ADMIN, so rule out the first here.
        if !self.server.caps.contains(crate::types::FsCaps::PASSTHROUGH) {
            return Err(crate::Error::ENOTSUP);
        }

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

    /// Tells the kernel to forget the cached attributes of an inode, keeping any
    /// cached data.
    ///
    /// macOS only honours this on a synchronous mount, and reports `ENOSYS`
    /// otherwise.
    pub async fn invalidate_attrs(&self, ino: Ino) -> std::result::Result<(), NotifyError> {
        self.supported(since::INVAL)?;

        self.server
            .dev
            .write_buf(InvalInode::attrs(ino).encode(self.cfg())?)
            .await
            .0?;
        Ok(())
    }

    /// Tells the kernel to forget an inode's cached attributes and data.
    pub async fn invalidate_inode(&self, ino: Ino) -> std::result::Result<(), NotifyError> {
        self.supported(since::INVAL)?;

        self.server
            .dev
            .write_buf(InvalInode::new(ino).encode(self.cfg())?)
            .await
            .0?;
        Ok(())
    }

    /// Tells the kernel to forget an inode's cached attributes and a range of
    /// its cached data.
    pub async fn invalidate_inode_range<R>(
        &self,
        ino: Ino,
        range: R,
    ) -> std::result::Result<(), NotifyError>
    where
        R: Into<FileRange>,
    {
        self.supported(since::INVAL)?;

        self.server
            .dev
            .write_buf(InvalInode::new(ino).range(range).encode(self.cfg())?)
            .await
            .0?;
        Ok(())
    }

    /// Tells the kernel a directory entry is gone, so it can notify anything
    /// watching the parent.
    ///
    /// Like [`invalidate_entry`](Self::invalidate_entry), this takes the parent
    /// directory's lock on Linux and must not be sent from a handler operating
    /// on that directory.
    pub async fn delete_inode<N>(
        &self,
        parent: Ino,
        child: Ino,
        name: N,
    ) -> std::result::Result<(), NotifyError>
    where
        N: AsRef<OsStr>,
    {
        let name = name.as_ref().as_bytes();
        let mut name_buf = self.buffer_pool().checkout_with_capacity(name.len());
        name_buf.extend_from_slice(name);

        self.supported(since::DELETE)?;

        self.server
            .dev
            .write_buf(Delete::new(parent, child, name_buf).encode(self.cfg())?)
            .await
            .0?;
        Ok(())
    }

    /// Tells the kernel to forget a cached directory entry.
    ///
    /// On Linux this takes the parent directory's lock, so sending it from a
    /// handler for an operation on that same directory deadlocks: queue it for
    /// after the reply instead.
    pub async fn invalidate_entry<N>(
        &self,
        parent: Ino,
        name: N,
    ) -> std::result::Result<(), NotifyError>
    where
        N: AsRef<OsStr>,
    {
        let name = name.as_ref().as_bytes();
        let mut name_buf = self.buffer_pool().checkout_with_capacity(name.len());
        name_buf.extend_from_slice(name);

        self.supported(since::INVAL)?;

        self.server
            .dev
            .write_buf(InvalEntry::new(parent, name_buf).encode(self.cfg())?)
            .await
            .0?;
        Ok(())
    }

    /// Marks a cached directory entry stale, so the kernel looks it up again
    /// rather than dropping it.
    ///
    /// Reports [`NotifyError::Unsupported`] on a kernel that predates this, which
    /// would otherwise treat it as a full invalidation. Takes the parent
    /// directory's lock on Linux, as
    /// [`invalidate_entry`](Self::invalidate_entry) does.
    pub async fn expire_entry<N>(
        &self,
        parent: Ino,
        name: N,
    ) -> std::result::Result<(), NotifyError>
    where
        N: AsRef<OsStr>,
    {
        let name = name.as_ref().as_bytes();
        let mut name_buf = self.buffer_pool().checkout_with_capacity(name.len());
        name_buf.extend_from_slice(name);

        self.supported(since::EXPIRE_ONLY)?;

        self.server
            .dev
            .write_buf(ExpireEntry::new(parent, name_buf).encode(self.cfg())?)
            .await
            .0?;
        Ok(())
    }

    /// Invalidates every cached directory entry at once.
    pub async fn increment_epoch(&self) -> std::result::Result<(), NotifyError> {
        self.supported(since::INC_EPOCH)?;

        self.server
            .dev
            .write_buf(IncrementEpoch::new().encode(self.cfg())?)
            .await
            .0?;
        Ok(())
    }

    /// Puts data straight into the kernel's page cache for an inode.
    pub async fn set_cache<B>(
        &self,
        ino: Ino,
        offset: u64,
        data: B,
    ) -> std::result::Result<(), NotifyError>
    where
        B: IntoIoBuf,
    {
        self.supported(since::STORE)?;

        self.server
            .dev
            .write_buf(Store::new(ino, offset, data).encode(self.cfg())?)
            .await
            .0?;
        Ok(())
    }

    /// Asks the kernel for data it has cached for an inode.
    pub async fn get_cache(
        &self,
        ino: Ino,
        offset: u64,
        len: usize,
    ) -> std::result::Result<CacheData, NotifyError> {
        struct Guard<'a> {
            replies: &'a crate::server::ReplyState,
            id: u32,
        }

        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                self.replies.pending.borrow_mut().remove(&self.id);
            }
        }

        self.supported(since::RETRIEVE)?;

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
            return Err(NotifyError::Io(crate::Error::EIO.into()));
        };

        std::mem::forget(guard);

        Ok(CacheData { reply: reply? })
    }
}

/// The protocol version each notification arrived in.
mod since {
    pub(super) const INVAL: u32 = 12;
    pub(super) const STORE: u32 = 15;
    pub(super) const RETRIEVE: u32 = 15;
    pub(super) const DELETE: u32 = 18;
    pub(super) const EXPIRE_ONLY: u32 = 38;
    pub(super) const INC_EPOCH: u32 = 44;
}

/// Refuses a notification the kernel is too old to understand, rather than
/// letting it reject the write or, worse, read the wrong fields.
///
/// `FUSE_HAS_EXPIRE_ONLY` is a capability as well as a version, but no kernel
/// below 7.38 offers it, so the version alone decides here, as it does in
/// `KernelCaps`.
fn supported(minor_ver: u32, since: u32) -> std::result::Result<(), NotifyError> {
    if minor_ver >= since {
        Ok(())
    } else {
        Err(NotifyError::Unsupported)
    }
}

impl Context {
    fn supported(&self, since: u32) -> std::result::Result<(), NotifyError> {
        supported(self.server.minor_ver, since)
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

#[cfg(test)]
mod tests {
    use super::*;

    // `Context` needs a live server, so the gate is called directly.
    fn is_supported(minor_ver: u32, since: u32) -> bool {
        super::supported(minor_ver, since).is_ok()
    }

    #[test]
    fn a_current_kernel_supports_every_notification() {
        let minor_ver = crate::handshake::MINOR_VER;

        for since in [
            since::INVAL,
            since::STORE,
            since::RETRIEVE,
            since::DELETE,
            since::EXPIRE_ONLY,
            since::INC_EPOCH,
        ] {
            assert!(is_supported(minor_ver, since));
        }
    }

    #[test]
    fn macos_supports_only_the_older_notifications() {
        // The macOS kernels speak 7.19.
        assert!(is_supported(19, since::INVAL));
        assert!(is_supported(19, since::STORE));
        assert!(is_supported(19, since::DELETE));
        assert!(!is_supported(19, since::EXPIRE_ONLY));
        assert!(!is_supported(19, since::INC_EPOCH));
    }

    #[test]
    fn an_ancient_kernel_supports_none_of_them() {
        assert!(!is_supported(11, since::INVAL));
        assert!(!is_supported(14, since::STORE));
        assert!(!is_supported(17, since::DELETE));
    }

    #[test]
    fn the_versions_are_in_the_order_they_were_added() {
        assert!(since::INVAL < since::STORE);
        assert!(since::STORE <= since::RETRIEVE);
        assert!(since::RETRIEVE < since::DELETE);
        assert!(since::DELETE < since::EXPIRE_ONLY);
        assert!(since::EXPIRE_ONLY < since::INC_EPOCH);
    }
}
