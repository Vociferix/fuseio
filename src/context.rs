use crate::Result;
use crate::async_rc::AsyncRc;
use crate::buf::{BufPool, IntoIoBuf};
use crate::conn::{Connection, ConnectionMeta};
use crate::fs::types::{NotifyError, PassthroughFd};
use crate::proto::notify::{
    Delete, EncodeNotify, ExpireEntry, IncrementEpoch, InvalEntry, InvalInode, Prune, Retrieve,
    Store,
};
use crate::proto::{Cfg, request::NotifyReply};
use crate::server::ServerInner;
use crate::types::{Feature, FileRange, FsCaps, Ino, Version};

use futures_util::{Stream, StreamExt};

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

pub struct Context<C> {
    pub(crate) server: AsyncRc<ServerInner<C>>,
}

#[derive(Debug, Clone)]
pub struct CacheData {
    reply: NotifyReply,
}

#[derive(Debug)]
pub struct NotifyPruneCache<C> {
    ctx: Context<C>,
    prune: Option<Prune>,
}

impl<C> Clone for Context<C> {
    fn clone(&self) -> Self {
        Self {
            server: self.server.clone(),
        }
    }
}

impl<C> Context<C> {
    pub(crate) fn new(server: AsyncRc<ServerInner<C>>) -> Self {
        Self { server }
    }

    pub fn proto_version(&self) -> Version {
        Version(crate::handshake::MAJOR_VER, self.server.minor_ver)
    }

    /// The features the filesystem enabled for this connection.
    pub fn caps(&self) -> FsCaps {
        self.server.caps
    }

    pub fn buffer_pool(&self) -> &BufPool {
        &self.server.buf_pool
    }
}

impl<C: Connection> Context<C> {
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
    pub async fn open_passthrough<T>(&self, fd: T) -> Result<PassthroughFd<T, C>>
    where
        T: std::os::fd::AsFd,
    {
        // The kernel reports EPERM whether the feature wasn't negotiated or the
        // process lacks CAP_SYS_ADMIN, so rule out the first here.
        if !self.supports(Feature::Passthrough) {
            return Err(crate::Error::ENOTSUP);
        }

        PassthroughFd::open(fd, self).await
    }

    pub(crate) async fn send_notif(
        &self,
        buf: impl IntoIoBuf,
    ) -> std::result::Result<(), NotifyError> {
        use compio::buf::{IoBuf, IoVectoredBuf};

        match buf.into_io_buf() {
            crate::buf::IoBuffer::Buf(buf) => {
                if buf.buf_len() > self.server.conn.max_notification_size() {
                    return Err(NotifyError::TooLarge);
                }
                self.server.conn.send_notification(buf).await.0?
            }
            crate::buf::IoBuffer::VecBuf(buf) => {
                if buf.total_len() > self.server.conn.max_notification_size() {
                    return Err(NotifyError::TooLarge);
                }
                self.server.conn.send_notification_vectored(buf).await.0?
            }
        }

        Ok(())
    }

    /// Tells the kernel to forget the cached attributes of an inode, keeping any
    /// cached data.
    ///
    /// macOS only honours this on a synchronous mount, and reports `ENOSYS`
    /// otherwise.
    pub async fn invalidate_attrs(&self, ino: Ino) -> std::result::Result<(), NotifyError> {
        self.require(Feature::InvalidateInode)?;

        self.send_notif(InvalInode::attrs(ino).encode(self.cfg())?)
            .await
    }

    /// Tells the kernel to forget an inode's cached attributes and data.
    pub async fn invalidate_inode(&self, ino: Ino) -> std::result::Result<(), NotifyError> {
        self.require(Feature::InvalidateInode)?;

        self.send_notif(InvalInode::new(ino).encode(self.cfg())?)
            .await
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

        self.send_notif(InvalInode::new(ino).range(range).encode(self.cfg())?)
            .await
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
        self.require(Feature::DeleteEntry)?;

        let name = name.as_ref().as_bytes();
        let mut name_buf = self.buffer_pool().checkout_with_capacity(name.len());
        name_buf.extend_from_slice(name);

        self.send_notif(Delete::new(parent, child, name_buf).encode(self.cfg())?)
            .await
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
        self.require(Feature::InvalidateEntry)?;

        let name = name.as_ref().as_bytes();
        let mut name_buf = self.buffer_pool().checkout_with_capacity(name.len());
        name_buf.extend_from_slice(name);

        self.send_notif(InvalEntry::new(parent, name_buf).encode(self.cfg())?)
            .await
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
        self.require(Feature::ExpireEntry)?;

        let name = name.as_ref().as_bytes();
        let mut name_buf = self.buffer_pool().checkout_with_capacity(name.len());
        name_buf.extend_from_slice(name);

        self.send_notif(ExpireEntry::new(parent, name_buf).encode(self.cfg())?)
            .await
    }

    /// Invalidates every cached directory entry at once.
    pub async fn increment_epoch(&self) -> std::result::Result<(), NotifyError> {
        self.require(Feature::IncrementEpoch)?;

        self.send_notif(IncrementEpoch::new().encode(self.cfg())?)
            .await
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

        self.send_notif(Store::new(ino, offset, data).encode(self.cfg())?)
            .await
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

        self.send_notif(Retrieve::new(id, ino, offset, len).encode(self.cfg())?)
            .await?;

        let Some(reply) = rx.await else {
            return Err(NotifyError::Io(crate::Error::EIO.into()));
        };

        std::mem::forget(guard);

        Ok(CacheData { reply: reply? })
    }
}

impl<C: ConnectionMeta> Context<C> {
    pub(crate) fn cfg(&self) -> Cfg {
        Cfg {
            minor_ver: self.server.minor_ver,
            flags: self.server.flags,
        }
    }

    /// Whether this connection allows a feature.
    ///
    /// Folds together the protocol version the kernel speaks, what was
    /// negotiated, and the platform, so a filesystem doesn't have to know which
    /// decides a given feature. Features that are purely negotiated are in
    /// [`caps`](Self::caps).
    pub fn supports(&self, feature: Feature) -> bool {
        crate::types::feature::supports(
            crate::types::PeerCaps::of(&self.server.conn),
            feature,
            self.server.minor_ver,
            self.server.caps,
        )
    }

    /// Refuses a notification this connection doesn't allow, rather than letting
    /// the kernel reject the write or, worse, read the wrong fields.
    fn require(&self, feature: Feature) -> std::result::Result<(), NotifyError> {
        if self.supports(feature) {
            Ok(())
        } else {
            Err(NotifyError::Unsupported)
        }
    }

    pub fn prune_cache(&self) -> std::result::Result<NotifyPruneCache<C>, NotifyError> {
        self.require(Feature::PruneCache)?;

        Ok(NotifyPruneCache {
            ctx: self.clone(),
            prune: None,
        })
    }
}

impl<C> std::fmt::Debug for Context<C> {
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

impl<C> NotifyPruneCache<C> {
    pub fn is_empty(&self) -> bool {
        self.prune.as_ref().is_none_or(Prune::is_empty)
    }

    pub fn len(&self) -> usize {
        self.prune.as_ref().map_or(0, Prune::len)
    }

    pub fn as_slice(&self) -> &[Ino] {
        self.prune.as_ref().map_or(&[], Prune::as_slice)
    }

    pub fn push(&mut self, ino: Ino) {
        self.prune
            .get_or_insert_with(|| Prune::with_capacity(self.ctx.buffer_pool(), 1))
            .push(ino);
    }

    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = Ino>,
    {
        let iter = iter.into_iter();
        let prune = self.prune.get_or_insert_with(|| {
            Prune::with_capacity(self.ctx.buffer_pool(), iter.size_hint().0)
        });
        iter.for_each(|ino| prune.push(ino));
    }

    pub fn try_extend<I, E>(&mut self, iter: I) -> std::result::Result<(), E>
    where
        I: IntoIterator<Item = std::result::Result<Ino, E>>,
    {
        let mut iter = iter.into_iter();
        let prune = self.prune.get_or_insert_with(|| {
            Prune::with_capacity(self.ctx.buffer_pool(), iter.size_hint().0)
        });
        iter.try_for_each(|ino| {
            prune.push(ino?);
            Ok(())
        })
    }

    pub async fn extend_stream<S>(&mut self, stream: S)
    where
        S: Stream<Item = Ino>,
    {
        let mut stream = std::pin::pin!(stream);
        let prune = self.prune.get_or_insert_with(|| {
            Prune::with_capacity(self.ctx.buffer_pool(), stream.size_hint().0)
        });
        while let Some(ino) = stream.next().await {
            prune.push(ino);
        }
    }

    pub async fn try_extend_stream<S, E>(&mut self, stream: S) -> std::result::Result<(), E>
    where
        S: Stream<Item = std::result::Result<Ino, E>>,
    {
        let mut stream = std::pin::pin!(stream);
        let prune = self.prune.get_or_insert_with(|| {
            Prune::with_capacity(self.ctx.buffer_pool(), stream.size_hint().0)
        });
        while let Some(ino) = stream.next().await {
            prune.push(ino?);
        }
        Ok(())
    }

    fn unpack_reserve(self, additional: usize) -> (Context<C>, Prune) {
        let Self { ctx, prune } = self;
        let prune = if let Some(mut prune) = prune {
            prune.reserve(additional);
            prune
        } else {
            Prune::with_capacity(ctx.buffer_pool(), additional)
        };
        (ctx, prune)
    }

    pub fn with_inodes<I>(self, iter: I) -> Self
    where
        I: IntoIterator<Item = Ino>,
    {
        let iter = iter.into_iter();
        let (ctx, mut prune) = self.unpack_reserve(iter.size_hint().0);
        iter.for_each(|ino| prune.push(ino));
        Self {
            ctx,
            prune: Some(prune),
        }
    }

    pub fn try_with_inodes<I, E>(self, iter: I) -> std::result::Result<Self, E>
    where
        I: IntoIterator<Item = std::result::Result<Ino, E>>,
    {
        let mut iter = iter.into_iter();
        let (ctx, mut prune) = self.unpack_reserve(iter.size_hint().0);
        iter.try_for_each(|ino| -> std::result::Result<(), E> {
            prune.push(ino?);
            Ok(())
        })?;
        Ok(Self {
            ctx,
            prune: Some(prune),
        })
    }

    pub async fn with_inodes_stream<S>(self, stream: S) -> Self
    where
        S: Stream<Item = Ino>,
    {
        let mut stream = std::pin::pin!(stream);
        let (ctx, mut prune) = self.unpack_reserve(stream.size_hint().0);
        while let Some(ino) = stream.next().await {
            prune.push(ino);
        }
        Self {
            ctx,
            prune: Some(prune),
        }
    }

    pub async fn try_with_inodes_stream<S, E>(self, stream: S) -> std::result::Result<Self, E>
    where
        S: Stream<Item = std::result::Result<Ino, E>>,
    {
        let mut stream = std::pin::pin!(stream);
        let (ctx, mut prune) = self.unpack_reserve(stream.size_hint().0);
        while let Some(ino) = stream.next().await {
            prune.push(ino?);
        }
        Ok(Self {
            ctx,
            prune: Some(prune),
        })
    }
}

impl<C: Connection> NotifyPruneCache<C> {
    pub async fn notify(self) -> std::result::Result<(), NotifyError> {
        let Self { ctx, prune } = self;

        if let Some(prune) = prune {
            ctx.send_notif(prune.encode(ctx.cfg())?).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // `Context` needs a live server, so the gate is asked through `Feature`,
    // against a peer that gates nothing.
    fn supported(feature: Feature, minor_ver: u32) -> bool {
        crate::types::feature::supports(
            crate::types::PeerCaps::PERMISSIVE,
            feature,
            minor_ver,
            FsCaps::all(),
        )
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
