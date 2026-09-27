use super::{
    AsFd, BufResult, Connection, IoBuf, IoBufMut, IoVectoredBuf, RawBackingId, Result,
    SharedConnection,
};

#[derive(Debug)]
pub struct DevFuseConn {
    _priv: (),
}

#[derive(Debug)]
pub struct DevFuseSharedConn {
    _priv: (),
}

impl Connection for DevFuseConn {
    type ReqToken = ();

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

impl SharedConnection for DevFuseSharedConn {
    type Bound = DevFuseConn;
    type ReqToken = ();

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
