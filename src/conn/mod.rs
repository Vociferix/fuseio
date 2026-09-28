#![allow(unused)] // TODO: delete me

use crate::buf::{IntoIoBuf, IoBuffer};

use std::io::Result;
use std::os::fd::AsFd;

use compio::buf::{BufResult, IoBuf, IoBufMut, IoVectoredBuf};

mod dev_fuse;

pub use crate::passthrough::RawBackingId;
pub use crate::types::{ConnCaps, NotifyCaps};

pub use dev_fuse::{DevFuseConn, DevFuseSharedConn};

pub trait ConnectionMeta: Sized + 'static {
    type ReqToken;

    fn capabilities(&self) -> ConnCaps;

    fn notify_capabilities(&self) -> NotifyCaps;

    fn max_response_size(&self) -> usize {
        const { u32::MAX as usize }
    }

    fn max_notification_size(&self) -> usize {
        const { u32::MAX as usize }
    }
}

pub trait SharedConnection: ConnectionMeta {
    type Bound: Connection;

    async fn bind(self) -> Result<Self::Bound>;

    async fn try_clone(&self) -> Result<Self>;

    async fn recv_request<B>(&self, buf: B) -> BufResult<(usize, Self::ReqToken), B>
    where
        B: IoBufMut;

    async fn send_response<B>(&self, token: Self::ReqToken, buf: B) -> BufResult<(), B>
    where
        B: IoBuf;
}

pub trait Connection: ConnectionMeta {
    async fn recv_request<B>(&self, buf: B) -> BufResult<(usize, Self::ReqToken), B>
    where
        B: IoBufMut;

    async fn send_response<B>(&self, token: Self::ReqToken, buf: B) -> BufResult<(), B>
    where
        B: IoBuf;

    async fn send_response_vectored<B>(&self, token: Self::ReqToken, buf: B) -> BufResult<(), B>
    where
        B: IoVectoredBuf;

    async fn discard(&self, token: Self::ReqToken) {
        let _ = (self, token);
    }

    async fn send_response_buf<B>(&self, token: Self::ReqToken, buf: B) -> Result<()>
    where
        B: IntoIoBuf,
    {
        match buf.into_io_buf() {
            IoBuffer::Buf(buf) => self.send_response(token, buf).await.0,
            IoBuffer::VecBuf(buf) => self.send_response_vectored(token, buf).await.0,
        }
    }

    async fn send_notification<B>(&self, buf: B) -> BufResult<(), B>
    where
        B: IoBuf;

    async fn send_notification_vectored<B>(&self, buf: B) -> BufResult<(), B>
    where
        B: IoVectoredBuf;

    async fn send_notification_buf<B>(&self, buf: B) -> Result<()>
    where
        B: IntoIoBuf,
    {
        match buf.into_io_buf() {
            IoBuffer::Buf(buf) => self.send_notification(buf).await.0,
            IoBuffer::VecBuf(buf) => self.send_notification_vectored(buf).await.0,
        }
    }

    async fn open_passthrough<T>(&self, fd: T) -> Result<RawBackingId>
    where
        T: AsFd,
    {
        let _ = fd;
        Err(std::io::ErrorKind::Unsupported.into())
    }

    fn close_passthrough(&self, backing_id: RawBackingId) -> Result<()> {
        let _ = backing_id;
        Err(std::io::ErrorKind::Unsupported.into())
    }
}

pub enum Conn<C: SharedConnection> {
    Shared(C),
    Bound(C::Bound),
}

impl<C> Clone for Conn<C>
where
    C: SharedConnection + Clone,
    C::Bound: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::Shared(conn) => Self::Shared(conn.clone()),
            Self::Bound(conn) => Self::Bound(conn.clone()),
        }
    }
}

impl<C> PartialEq for Conn<C>
where
    C: SharedConnection + PartialEq,
    C::Bound: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Shared(l), Self::Shared(r)) => l == r,
            (Self::Bound(l), Self::Bound(r)) => l == r,
            _ => false,
        }
    }
}

impl<C> Eq for Conn<C>
where
    C: SharedConnection + Eq,
    C::Bound: Eq,
{
}

impl<C> PartialOrd for Conn<C>
where
    C: SharedConnection + PartialOrd,
    C::Bound: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Self::Shared(l), Self::Shared(r)) => l.partial_cmp(r),
            (Self::Bound(l), Self::Bound(r)) => l.partial_cmp(r),
            (Self::Shared(_), _) => Some(std::cmp::Ordering::Less),
            _ => Some(std::cmp::Ordering::Greater),
        }
    }
}

impl<C> Ord for Conn<C>
where
    C: SharedConnection + Ord,
    C::Bound: Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::Shared(l), Self::Shared(r)) => l.cmp(r),
            (Self::Bound(l), Self::Bound(r)) => l.cmp(r),
            (Self::Shared(_), _) => std::cmp::Ordering::Less,
            _ => std::cmp::Ordering::Greater,
        }
    }
}

impl<C> std::hash::Hash for Conn<C>
where
    C: SharedConnection + std::hash::Hash,
    C::Bound: std::hash::Hash,
{
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        match self {
            Self::Shared(conn) => (false, conn).hash(state),
            Self::Bound(conn) => (false, conn).hash(state),
        }
    }
}

impl<C, T> AsRef<T> for Conn<C>
where
    C: SharedConnection + AsRef<T>,
    C::Bound: AsRef<T>,
    T: ?Sized,
{
    fn as_ref(&self) -> &T {
        match self {
            Self::Shared(conn) => conn.as_ref(),
            Self::Bound(conn) => conn.as_ref(),
        }
    }
}

impl<C, T> AsMut<T> for Conn<C>
where
    C: SharedConnection + AsMut<T>,
    C::Bound: AsMut<T>,
    T: ?Sized,
{
    fn as_mut(&mut self) -> &mut T {
        match self {
            Self::Shared(conn) => conn.as_mut(),
            Self::Bound(conn) => conn.as_mut(),
        }
    }
}

impl<C> std::ops::Deref for Conn<C>
where
    C: SharedConnection + std::ops::Deref,
    C::Bound: std::ops::Deref<Target = C::Target>,
{
    type Target = C::Target;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Shared(conn) => conn.deref(),
            Self::Bound(conn) => conn.deref(),
        }
    }
}

impl<C> std::ops::DerefMut for Conn<C>
where
    C: SharedConnection + std::ops::DerefMut,
    C::Bound: std::ops::DerefMut<Target = C::Target>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Shared(conn) => conn.deref_mut(),
            Self::Bound(conn) => conn.deref_mut(),
        }
    }
}

impl<C> std::os::fd::AsFd for Conn<C>
where
    C: SharedConnection + std::os::fd::AsFd,
    C::Bound: std::os::fd::AsFd,
{
    fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
        match self {
            Self::Shared(conn) => conn.as_fd(),
            Self::Bound(conn) => conn.as_fd(),
        }
    }
}

impl<C> std::os::fd::AsRawFd for Conn<C>
where
    C: SharedConnection + std::os::fd::AsRawFd,
    C::Bound: std::os::fd::AsRawFd,
{
    fn as_raw_fd(&self) -> std::os::fd::RawFd {
        match self {
            Self::Shared(conn) => conn.as_raw_fd(),
            Self::Bound(conn) => conn.as_raw_fd(),
        }
    }
}

impl<C> std::os::fd::IntoRawFd for Conn<C>
where
    C: SharedConnection + std::os::fd::IntoRawFd,
    C::Bound: std::os::fd::IntoRawFd,
{
    fn into_raw_fd(self) -> std::os::fd::RawFd {
        match self {
            Self::Shared(conn) => conn.into_raw_fd(),
            Self::Bound(conn) => conn.into_raw_fd(),
        }
    }
}

impl<C> From<Conn<C>> for std::os::fd::OwnedFd
where
    C: SharedConnection + Into<std::os::fd::OwnedFd>,
    C::Bound: Into<std::os::fd::OwnedFd>,
{
    fn from(conn: Conn<C>) -> Self {
        match conn {
            Conn::Shared(conn) => conn.into(),
            Conn::Bound(conn) => conn.into(),
        }
    }
}

/// Wraps a `Connection::ReqToken` to check linear usage in debug builds.
///
/// In debug builds, this type panics on drop without calling `into_inner`.
/// Rust does not support linear types (yet) so this is just a debugging
/// helper to detect tokens getting dropped without being returned to the
/// connection.
#[repr(transparent)]
pub(crate) struct TokenGuard<T>(T);

impl<T> TokenGuard<T> {
    pub(crate) fn new(token: T) -> Self {
        Self(token)
    }

    pub(crate) fn into_inner(self) -> T {
        let guard = std::mem::ManuallyDrop::new(self);
        unsafe { std::ptr::read(&guard.0) }
    }
}

impl<T> Drop for TokenGuard<T> {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        {
            if !std::thread::panicking() {
                panic!("ReqToken was not returned to connection");
            }
        }
        log::error!("ReqToken was not returned to connection");
    }
}
