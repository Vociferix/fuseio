use crate::Result;
use crate::buf::{BufPool, IntoIoBuf};
use crate::dev_fuse::FuseChannel;
use crate::fs::types::{NotifyError, PassthroughFd};
use crate::proto::notify::{
    Delete, EncodeNotify, ExpireEntry, IncrementEpoch, InvalEntry, InvalInode, Retrieve, Store,
};
use crate::proto::request::{Cfg, NotifyReply};
use crate::server::ServerInner;
use crate::types::{Feature, FileRange, FsCaps, Ino, Version};

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

    /// The features the filesystem enabled for this connection.
    pub fn caps(&self) -> FsCaps {
        self.server.caps
    }

    /// Whether this connection allows a feature.
    ///
    /// Folds together the protocol version the kernel speaks, what was
    /// negotiated, and the platform, so a filesystem doesn't have to know which
    /// decides a given feature. Features that are purely negotiated are in
    /// [`caps`](Self::caps).
    pub fn supports(&self, feature: Feature) -> bool {
        crate::types::feature::supports(feature, self.server.minor_ver, self.server.caps)
    }

    /// Hands a file to the kernel, so it serves reads and writes from it
    /// directly instead of sending them to this filesystem.
    ///
    /// Only Linux has this, from 6.9, and only when built with
    /// `CONFIG_FUSE_PASSTHROUGH`. Whether it will work can't be established
    /// ahead of the attempt, so each failure says which requirement went unmet:
    ///
    /// - `ENOTSUP`: this platform or this connection has no passthrough. Either
    ///   [`Feature::Passthrough`](crate::types::Feature::Passthrough) isn't
    ///   allowed here, which this checks before asking the kernel, or the kernel
    ///   was built without `CONFIG_FUSE_PASSTHROUGH`.
    /// - `EPERM`: the process lacks `CAP_SYS_ADMIN` **in the initial user
    ///   namespace**. Being root inside a user namespace doesn't count, and a
    ///   capability check in the filesystem would report it as held, so this
    ///   error is the only reliable answer.
    /// - `EISDIR`, or `EINVAL` for anything else: passthrough covers regular
    ///   files only.
    /// - `ELOOP`: the file's own filesystem is already stacked as deeply as this
    ///   connection allows.
    /// - `EBADF`: the file descriptor isn't open.
    pub fn open_passthrough<T>(&self, fd: T) -> Result<PassthroughFd<T>>
    where
        T: std::os::fd::AsFd,
    {
        // The kernel reports EPERM whether the feature wasn't negotiated or the
        // process lacks CAP_SYS_ADMIN, so rule out the first here.
        if !self.supports(Feature::Passthrough) {
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
        self.require(Feature::InvalidateInode)?;

        self.server
            .dev
            .write_buf(InvalInode::attrs(ino).encode(self.cfg())?)
            .await
            .0?;
        Ok(())
    }

    /// Tells the kernel to forget an inode's cached attributes and data.
    pub async fn invalidate_inode(&self, ino: Ino) -> std::result::Result<(), NotifyError> {
        self.require(Feature::InvalidateInode)?;

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
        self.require(Feature::InvalidateInode)?;

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

        self.require(Feature::DeleteEntry)?;

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

        self.require(Feature::InvalidateEntry)?;

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

        self.require(Feature::ExpireEntry)?;

        self.server
            .dev
            .write_buf(ExpireEntry::new(parent, name_buf).encode(self.cfg())?)
            .await
            .0?;
        Ok(())
    }

    /// Invalidates every cached directory entry at once.
    pub async fn increment_epoch(&self) -> std::result::Result<(), NotifyError> {
        self.require(Feature::IncrementEpoch)?;

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
        self.require(Feature::StoreCache)?;

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

        self.require(Feature::RetrieveCache)?;

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

impl Context {
    /// Refuses a notification this connection doesn't allow, rather than letting
    /// the kernel reject the write or, worse, read the wrong fields.
    fn require(&self, feature: Feature) -> std::result::Result<(), NotifyError> {
        if self.supports(feature) {
            Ok(())
        } else {
            Err(NotifyError::Unsupported)
        }
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

    // `Context` needs a live server, so the gate is asked through `Feature`.
    fn supported(feature: Feature, minor_ver: u32) -> bool {
        crate::types::feature::supports(feature, minor_ver, FsCaps::all())
    }

    #[test]
    fn a_current_kernel_takes_every_notification() {
        let minor_ver = crate::handshake::MINOR_VER;

        for feature in [
            Feature::InvalidateInode,
            Feature::InvalidateEntry,
            Feature::DeleteEntry,
            Feature::ExpireEntry,
            Feature::StoreCache,
            Feature::RetrieveCache,
            Feature::IncrementEpoch,
        ] {
            assert!(supported(feature, minor_ver), "{feature:?}");
        }
    }

    // The macOS kernels speak 7.19.
    #[test]
    fn a_macos_kernel_takes_only_the_older_notifications() {
        assert!(supported(Feature::InvalidateInode, 19));
        assert!(supported(Feature::StoreCache, 19));
        assert!(supported(Feature::DeleteEntry, 19));
        assert!(!supported(Feature::ExpireEntry, 19));
        assert!(!supported(Feature::IncrementEpoch, 19));
    }

    #[test]
    fn an_ancient_kernel_takes_none_of_them() {
        assert!(!supported(Feature::InvalidateInode, 11));
        assert!(!supported(Feature::StoreCache, 14));
        assert!(!supported(Feature::DeleteEntry, 17));
    }
}
