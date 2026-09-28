use super::{
    AsFd, BufResult, ConnCaps, Connection, ConnectionMeta, IoBuf, IoBufMut, IoVectoredBuf,
    NotifyCaps, RawBackingId, Result, SharedConnection,
};

#[derive(Debug)]
pub struct DevFuseConn {
    _priv: (),
}

#[derive(Debug)]
pub struct DevFuseSharedConn {
    _priv: (),
}

#[cfg(target_os = "linux")]
const CAPS: ConnCaps = ConnCaps::PASSTHROUGH
    .union(ConnCaps::INDEPENDENT_CLONES)
    .union(ConnCaps::ABORT)
    .union(ConnCaps::SYNCFS)
    .union(ConnCaps::RENAME2)
    .union(ConnCaps::POLL)
    .union(ConnCaps::MAX_PAGES);

#[cfg(any(
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly",
))]
const CAPS: ConnCaps = ConnCaps::empty();

#[cfg(target_os = "macos")]
const CAPS: ConnCaps = ConnCaps::RENAME2
    .union(ConnCaps::POLL)
    .union(ConnCaps::MONITOR)
    .union(ConnCaps::VOLUME_NAME)
    .union(ConnCaps::BACKUP_TIMES)
    .union(ConnCaps::EXCHANGE_DATA)
    .union(ConnCaps::MAX_PAGES);

#[cfg(target_os = "linux")]
const NOTIFY_CAPS: NotifyCaps = NotifyCaps::all();

#[cfg(any(
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly",
))]
const NOTIFY_CAPS: NotifyCaps = NotifyCaps::INVAL_INODE.union(NotifyCaps::INVAL_ENTRY);

#[cfg(target_os = "macos")]
const NOTIFY_CAPS: NotifyCaps = NotifyCaps::INVAL_INODE
    .union(NotifyCaps::INVAL_ENTRY)
    .union(NotifyCaps::EXPIRE_ENTRY)
    .union(NotifyCaps::INC_EPOCH)
    .union(NotifyCaps::DELETE)
    .union(NotifyCaps::STORE)
    .union(NotifyCaps::RETRIEVE)
    .union(NotifyCaps::POLL_WAKEUP);

impl ConnectionMeta for DevFuseConn {
    type ReqToken = ();

    fn capabilities(&self) -> ConnCaps {
        CAPS
    }

    fn notify_capabilities(&self) -> NotifyCaps {
        NOTIFY_CAPS
    }
}

impl Connection for DevFuseConn {
    async fn recv_request<B>(&self, buf: B) -> BufResult<(usize, Self::ReqToken), B>
    where
        B: IoBufMut,
    {
        todo!()
    }

    async fn send_response<B>(&self, token: Self::ReqToken, buf: B) -> BufResult<(), B>
    where
        B: IoBuf,
    {
        todo!()
    }

    async fn send_response_vectored<B>(&self, token: Self::ReqToken, buf: B) -> BufResult<(), B>
    where
        B: IoVectoredBuf,
    {
        todo!()
    }

    async fn send_notification<B>(&self, buf: B) -> BufResult<(), B>
    where
        B: IoBuf,
    {
        self.send_response((), buf).await
    }

    async fn send_notification_vectored<B>(&self, buf: B) -> BufResult<(), B>
    where
        B: IoVectoredBuf,
    {
        self.send_response_vectored((), buf).await
    }

    #[cfg(target_os = "linux")]
    async fn open_passthrough<T>(&self, fd: T) -> Result<RawBackingId>
    where
        T: AsFd,
    {
        todo!()
    }

    #[cfg(target_os = "linux")]
    fn close_passthrough(&self, backing_id: RawBackingId) -> Result<()> {
        todo!()
    }
}

impl ConnectionMeta for DevFuseSharedConn {
    type ReqToken = ();

    fn capabilities(&self) -> ConnCaps {
        CAPS
    }

    fn notify_capabilities(&self) -> NotifyCaps {
        NOTIFY_CAPS
    }
}

impl SharedConnection for DevFuseSharedConn {
    type Bound = DevFuseConn;

    async fn bind(self) -> Result<Self::Bound> {
        todo!()
    }

    async fn try_clone(&self) -> Result<Self> {
        todo!()
    }

    async fn recv_request<B>(&self, buf: B) -> BufResult<(usize, Self::ReqToken), B>
    where
        B: IoBufMut,
    {
        todo!()
    }

    async fn send_response<B>(&self, token: Self::ReqToken, buf: B) -> BufResult<(), B>
    where
        B: IoBuf,
    {
        todo!()
    }
}
