use crate::buf::{IntoIoBuf, IoBuffer};

use std::io::Result;
use std::os::fd::AsFd;

use compio::buf::{BufResult, IoBuf, IoBufMut, IoVectoredBuf};

mod dev_fuse;

pub use crate::passthrough::RawBackingId;

pub use dev_fuse::{DevFuseConn, DevFuseSharedConn};

pub trait SharedConnection: Sized + 'static {
    type Bound: Connection;

    // This doesn't necessarily have to be the same as `<Self::Bound as Connection>::ReqToken`.
    // This token is only used for initialization, so if the connection would benefit from
    // having a distinct token type for the handshake than for normal operation, that is permitted.
    type ReqToken;

    async fn bind(self) -> Result<Self::Bound>;

    async fn try_clone(&self) -> Result<Self>;

    async fn recv_request<B>(&self, buf: B) -> BufResult<(usize, Self::ReqToken), B>
    where
        B: IoBufMut;

    async fn send_response<B>(&self, token: Self::ReqToken, buf: B) -> BufResult<(), B>
    where
        B: IoBuf;
}

pub trait Connection: Sized + 'static {
    type ReqToken;

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
        Err(std::io::ErrorKind::Unsupported.into())
    }

    fn close_passthrough(&self, backing_id: RawBackingId) -> Result<()> {
        Err(std::io::ErrorKind::Unsupported.into())
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
