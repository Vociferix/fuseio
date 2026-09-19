use crate::async_rc::AsyncRc;
use crate::buf::{Buf, BufPool, IntoIoBuf};
use crate::dev_fuse::FuseChannel;
use crate::fs::{BindFs, Fs};
use crate::handle::{Handle, Once};
use crate::mount::Unmount;
use crate::proto::request::{AnyRequest, Body, NotifyReply, SharedNotifyReply};
use crate::types::ReplyInitFlags;

use compio::BufResult;
use compio::runtime::JoinHandle;
use crossfire::{AsyncRx, MAsyncTx, mpsc::Array};

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io::Result;
use std::os::fd::AsFd;
use std::rc::Rc;
use std::sync::Arc;

pub struct Server<F, U> {
    pub inner: Rc<ServerInner>,
    pub fs: F,
    pub once: Option<Once<U>>,
}

pub struct ServerInner {
    pub id: usize,
    pub dev: FuseChannel,
    pub minor_ver: u32,
    pub flags: ReplyInitFlags,
    pub buf_pool: BufPool,
    pub buf_size: usize,
    pub open_reqs: RefCell<HashMap<u64, JoinHandle<()>>>,
    pub replies: ReplyState,
    pub mesh_rx: AsyncRx<Array<Message>>,
    pub mesh_tx: Arc<[MAsyncTx<Array<Message>>]>,
}

pub struct ReplyState {
    pub pending: RefCell<HashMap<u32, unsync::oneshot::Sender<NotifyReply>>>,
    pub next_id: Cell<u32>,
}

pub(crate) enum Message {
    Shutdown,
    Interrupt(u64),
    Reply(u32, SharedNotifyReply),
}

impl<F, U> std::ops::Deref for Server<F, U> {
    type Target = ServerInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<F, U> Server<F, U>
where
    F: crate::fs::Fs,
    U: Unmount,
{
    pub(crate) async fn new<B>(h: Handle<B, U>) -> std::io::Result<AsyncRc<Self>>
    where
        B: BindFs<BoundFs = F>,
        F: Fs,
    {
        Ok(AsyncRc::new(Self {
            inner: Rc::new(ServerInner {
                id: h.id,
                dev: h.dev.bind()?,
                minor_ver: h.minor_ver,
                flags: h.flags,
                buf_pool: BufPool::new(),
                buf_size: crate::MAX_WRITE_SIZE,
                open_reqs: RefCell::new(HashMap::new()),
                replies: ReplyState {
                    pending: RefCell::new(HashMap::new()),
                    next_id: Cell::new(0),
                },
                mesh_rx: h.mesh_rx,
                mesh_tx: h.mesh_tx,
            }),
            fs: h.fs.bind().await?,
            once: h.once,
        }))
    }

    pub(crate) async fn serve_requests(this: &AsyncRc<Self>) {
        while Self::serve_one(this).await {}
    }

    pub(crate) async fn unmount(this: AsyncRc<Self>) -> Result<()> {
        let Self {
            inner, fs, once, ..
        } = this.unwrap().await;

        fs.unmount().await;

        if let Some(once) = once {
            once.unmount
                .unmount(inner.dev.as_fd(), &once.path, &once.opts)
                .await?;
        }

        Ok(())
    }

    async fn serve_one(this: &AsyncRc<Self>) -> bool {
        match select_biased(
            Self::serve_messages(this),
            this.dev
                .read(this.buf_pool.checkout_with_capacity(this.buf_size)),
        )
        .await
        {
            either::Left(()) => false,
            either::Right(BufResult(Ok(len), mut buf)) => {
                unsafe {
                    buf.set_len(len);
                }

                Self::handle_req(this, buf)
            }
            either::Right(BufResult(Err(err), _)) => {
                Self::handle_error(this, err);
                true
            }
        }
    }

    async fn serve_messages(this: &AsyncRc<Self>) {
        loop {
            match this.mesh_rx.recv().await {
                Ok(Message::Shutdown) | Err(_) => return,
                Ok(Message::Interrupt(id)) => {
                    if let Some(task) = this.open_reqs.borrow_mut().remove(&id) {
                        drop(task.cancel());
                    }
                }
                Ok(Message::Reply(id, reply)) => {
                    let reply = reply.bind(&this.inner.buf_pool);
                    drop(this.inner.buf_pool.checkout::<u8>().steal());
                    Self::handle_reply(this, id, reply);
                }
            }
        }
    }

    fn handle_reply(&self, id: u32, reply: NotifyReply) {
        if let Some(tx) = self.inner.replies.pending.borrow_mut().remove(&id) {
            let _ = tx.send(reply);
        }
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
        if buf.len() < std::mem::size_of::<crate::proto::request::RawHeader>() {
            return true;
        }

        let unique = unsafe { (*(buf.as_ptr() as *const crate::proto::request::RawHeader)).unique };

        let AnyRequest { req, body } = match AnyRequest::decode(buf, this.minor_ver, this.flags) {
            Ok(req) => req,
            Err(err) => {
                let this = this.clone();
                compio::runtime::spawn(async move {
                    let _ = this.dev.write_buf(err.into_reply(unique)).await;
                })
                .detach();
                return true;
            }
        };

        match body {
            Body::Interrupt(intr) => {
                if let Some(task) = this.open_reqs.borrow_mut().remove(&intr.id()) {
                    drop(task.cancel());
                } else {
                    let this = this.clone();
                    let id = intr.id();
                    compio::runtime::spawn(async move {
                        for tx in this
                            .mesh_tx
                            .iter()
                            .enumerate()
                            .filter_map(|(idx, tx)| (idx != this.id).then_some(tx))
                        {
                            let _ = tx.send(Message::Interrupt(id)).await;
                        }
                    })
                    .detach();
                }
            }
            Body::NotifyReply(notify) => {
                let id = req.id();
                let worker = (id >> 32) as usize;
                let id = id as u32;
                if worker == this.inner.id {
                    Self::handle_reply(this, id, notify);
                } else {
                    let this = this.clone();
                    compio::runtime::spawn(async move {
                        let _ = this.mesh_tx[worker]
                            .send(Message::Reply(id, notify.share()))
                            .await;
                    })
                    .detach();
                }
            }
            body => {
                let this = this.clone();
                compio::runtime::spawn(async move {
                    this.handle_fs_op(req, body).await;
                })
                .detach();
            }
        }

        true
    }

    async fn handle_fs_op(&self, req: crate::types::Request, body: Body) {
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

impl ReplyState {
    pub fn channel(&self) -> (u32, unsync::oneshot::Receiver<NotifyReply>) {
        let id = self.next_id.get();
        self.next_id.set(id.wrapping_add(1));

        let (tx, rx) = unsync::oneshot::channel();

        self.pending.borrow_mut().insert(id, tx);

        (id, rx)
    }
}
