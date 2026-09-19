use crate::async_rc::AsyncRc;
use crate::buf::{Buf, BufPool, IntoIoBuf};
use crate::dev_fuse::FuseChannel;
use crate::fs::{BindFs, Fs};
use crate::handle::{Handle, Once};
use crate::mount::Unmount;
use crate::proto::request::{AnyRequest, Body, NotifyReply};
use crate::types::ReplyInitFlags;

use aligned_vec::{AVec, ConstAlign};
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
    pub inner: ServerInner,
    pub fs: F,
    pub once: Option<Once<U>>,
}

pub struct ServerInner {
    pub id: usize,
    pub dev: Rc<FuseChannel>,
    pub minor_ver: u32,
    pub flags: ReplyInitFlags,
    pub buf_pool: BufPool,
    pub buf_size: usize,
    pub open_reqs: RefCell<HashMap<u64, JoinHandle<()>>>,
    pub replies: Rc<ReplyState>,
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
    Reply(u64, AVec<u8, ConstAlign<8>>),
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
            inner: ServerInner {
                id: h.id,
                dev: Rc::new(h.dev.bind()?),
                minor_ver: h.minor_ver,
                flags: h.flags,
                buf_pool: BufPool::new(),
                buf_size: crate::MAX_WRITE_SIZE,
                open_reqs: RefCell::new(HashMap::new()),
                replies: Rc::new(ReplyState {
                    pending: RefCell::new(HashMap::new()),
                    next_id: Cell::new(0),
                }),
                mesh_rx: h.mesh_rx,
                mesh_tx: h.mesh_tx,
            },
            fs: h.fs.bind().await?,
            once: h.once,
        }))
    }

    pub(crate) async fn serve_requests(this: &AsyncRc<Self>) {
        while Self::serve_one(this).await {}
    }

    pub(crate) async fn unmount(this: AsyncRc<Self>) -> Result<()> {
        let Self {
            inner: ServerInner { dev, .. },
            fs,
            once,
            ..
        } = this.unwrap().await;

        fs.unmount().await;

        if let Some(once) = once {
            once.unmount
                .unmount(dev.as_fd(), &once.path, &once.opts)
                .await?;
        }

        Ok(())
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
                    drop(task.cancel());
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

    fn handle_reply(&self, id: u64, data: AVec<u8, ConstAlign<8>>) {
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
                todo!()
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
