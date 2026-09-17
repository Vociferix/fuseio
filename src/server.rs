use crate::async_rc::AsyncRc;
use crate::buf::{Buf, BufPool};
use crate::dev_fuse::FuseChannel;
use crate::fs::{BindFs, Fs};
use crate::handle::{Handle, Once};
use crate::mount::Unmount;

use compio::BufResult;
use compio::runtime::JoinHandle;
use crossfire::{AsyncRx, MAsyncTx, mpsc::Array};

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Result;
use std::rc::Rc;
use std::sync::Arc;

pub struct Server<F, U> {
    id: usize,
    dev: FuseChannel,
    fs: F,
    buf_pool: BufPool,
    buf_size: usize,
    open_reqs: RefCell<HashMap<u64, JoinHandle<()>>>,
    mesh_rx: AsyncRx<Array<Message>>,
    mesh_tx: Arc<[MAsyncTx<Array<Message>>]>,
    once: Option<Once<U>>,
}

pub(crate) struct SharedServer {
    mesh_tx: Vec<MAsyncTx<Array<Message>>>,
}

pub(crate) enum Message {
    Shutdown,
    Interrupt(u64),
    Reply(u64, Vec<u8>),
}

impl<F, U> Server<F, U> {
    pub(crate) fn new<B>(h: Handle<B, U>) -> AsyncRc<Self>
    where
        B: BindFs<BoundFs = F>,
        F: Fs,
    {
        todo!()
    }

    pub(crate) async fn serve_requests(this: &AsyncRc<Self>) {
        while Self::serve_one(this).await {}
    }

    pub(crate) async fn unmount(this: AsyncRc<Self>) -> Result<()> {
        todo!()
    }

    async fn serve_one(this: &AsyncRc<Self>) -> bool {
        match select_biased(
            this.mesh_rx.recv(),
            this.dev
                .read(this.buf_pool.checkout_with_capacity(this.buf_size)),
        )
        .await
        {
            either::Left(Ok(Message::Shutdown) | Err(_)) => return false,
            either::Left(Ok(Message::Interrupt(id))) => {
                if let Some(task) = this.open_reqs.borrow_mut().remove(&id) {
                    let _ = task.cancel();
                }
            }
            either::Left(Ok(Message::Reply(id, data))) => {
                Self::handle_reply(this, id, data);
            }
            either::Right(BufResult(Ok(len), mut buf)) => {
                unsafe {
                    buf.set_len(len);
                }

                return Self::handle_req(this, buf);
            }
            either::Right(BufResult(Err(err), _)) => {
                Self::handle_error(this, err);
            }
        }

        true
    }

    fn handle_reply(&self, id: u64, data: Vec<u8>) {
        todo!()
    }

    fn handle_error(this: &AsyncRc<Self>, err: std::io::Error) {
        todo!()
    }

    // TODO: enforce operation-gating `FsCaps` on every platform before
    // dispatching: reject `RenameMode`s not enabled (EINVAL on Linux/BSD,
    // ENOTSUP on macOS; `WhiteoutNoReplace` needs both RENAME_WHITEOUT and
    // RENAME_NOREPLACE, `ExchangeData` needs EXCHANGE_DATA), and FALLOCATE
    // (ENOSYS). Also translate ENOSYS returned for a non-`Replace` rename to
    // the same errno, since ENOSYS on RENAME2 makes Linux disable all flagged
    // renames for the mount.
    fn handle_req(this: &AsyncRc<Self>, buf: Buf) -> bool {
        todo!()
    }
}

fn select_biased<A, B>(a: A, b: B) -> impl Future<Output = either::Either<A::Output, B::Output>>
where
    A: Future,
    B: Future,
{
    use std::pin::Pin;
    use std::task::{Context, Poll};

    struct SelectBiased<A, B> {
        a: A,
        b: B,
    }

    impl<A, B> Future for SelectBiased<A, B>
    where
        A: Future,
        B: Future,
    {
        type Output = either::Either<A::Output, B::Output>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = unsafe { Pin::get_unchecked_mut(self) };

            let a = unsafe { Pin::new_unchecked(&mut this.a) };
            let b = unsafe { Pin::new_unchecked(&mut this.b) };

            if let Poll::Ready(a) = a.poll(cx) {
                return Poll::Ready(either::Left(a));
            }

            if let Poll::Ready(b) = b.poll(cx) {
                return Poll::Ready(either::Right(b));
            }

            Poll::Pending
        }
    }

    SelectBiased { a, b }
}
