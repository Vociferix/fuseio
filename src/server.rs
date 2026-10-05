use crate::Error;
use crate::access::Access;
use crate::async_rc::AsyncRc;
use crate::buf::{Buf, BufPool, IntoIoBuf};
use crate::cancel_token::CancelToken;
use crate::conn::{Conn, Connection, ConnectionMeta, SharedConnection, TokenGuard};
use crate::fs::req::{self, Req};
use crate::fs::types::{DirEntryBuf, DirEntryPlusBuf, XattrKeyBuf};
use crate::fs::{BindFs, Fs};
use crate::handle::{Handle, Once};
use crate::mount::Unmount;
use crate::proto::request::{self, AnyRequest, Body, NotifyReply, Opcode, SharedNotifyReply};
use crate::proto::response::{
    Bmap, CopyFileRange, Data, EncodeResp, IoctlReply, Lseek, Poll, Write, XattrLen,
};
use crate::types::{FsCaps, RenameMode, ReplyInitFlags, Request};

use compio::BufResult;
use crossfire::{AsyncRx, MAsyncTx, mpsc::Array};

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::io::Result;
use std::ops::ControlFlow;
use std::sync::Arc;

/// The default `max_write`, which also sets the size of every read buffer.
///
/// Linux won't send more than `max_pages` (256 by default, so 1 MiB) in one
/// write however large this is, and libfuse defaults to the same. FreeBSD has no
/// page limit and chunks writes by `max_write` alone, so raising this raises its
/// write size too.
pub const MAX_WRITE_SIZE: usize = 1024 * 1024;

/// `FUSE_BUFFER_HEADER_SIZE`: the room a request needs on top of `max_write`.
///
/// Linux refuses to fill a buffer smaller than a request header plus a write
/// header plus `max_write`, so this has to cover both headers.
pub const BUF_HEADER_SIZE: usize = 4096;

const _: () = assert!(
    BUF_HEADER_SIZE >= std::mem::size_of::<crate::proto::request::RawHeader>() + WRITE_HEADER_SIZE
);

/// `sizeof(struct fuse_write_in)`.
const WRITE_HEADER_SIZE: usize = 40;

pub struct Server<F, U: Unmount> {
    pub inner: AsyncRc<ServerInner<U::Conn>>,
    pub fs: F,
    pub once: Option<Once<U>>,
}

pub struct ServerInner<C> {
    pub id: usize,
    pub access: Access,
    pub conn: C,
    pub minor_ver: u32,
    pub flags: ReplyInitFlags,
    pub caps: FsCaps,
    pub ignore_interrupts: bool,
    pub buf_pool: BufPool,
    pub buf_size: usize,
    pub open_reqs: RefCell<HashMap<u64, CancelToken>>,
    pub early_interrupts: RefCell<EarlyInterrupts>,
    pub cancel_tokens: RefCell<Vec<CancelToken>>,
    pub replies: ReplyState,
    pub mesh_rx: AsyncRx<Array<Message>>,
    pub mesh_tx: Arc<[MAsyncTx<Array<Message>>]>,
}

pub struct ReplyState {
    pub pending: RefCell<HashMap<u32, unsync::oneshot::Sender<crate::Result<NotifyReply>>>>,
    pub next_id: Cell<u32>,
}

#[derive(Clone)]
pub(crate) enum Message {
    Shutdown,
    Interrupt(u64),
    Reply(u32, crate::Result<SharedNotifyReply>),
}

/// Interrupts for requests this worker hasn't seen yet.
///
/// An interrupt can reach a worker before the request it interrupts, either
/// because another worker is still decoding it or because the interrupt was
/// relayed. The id is held here until the request shows up, and the request
/// cancels itself the moment it does.
///
/// Ids the kernel never follows with a request (an interrupt that lost a race
/// with the reply) would otherwise accumulate, so the oldest is dropped once
/// the set is full. Dropping one only means that interrupt is not honoured,
/// which is what happened before it was recorded at all.
#[derive(Debug, Default)]
pub struct EarlyInterrupts {
    ids: VecDeque<u64>,
}

/// How many interrupts a worker remembers for requests it hasn't seen yet.
const MAX_EARLY_INTERRUPTS: usize = 64;

impl EarlyInterrupts {
    fn record(&mut self, id: u64) {
        if self.ids.contains(&id) {
            return;
        }

        if self.ids.len() == MAX_EARLY_INTERRUPTS {
            let dropped = self.ids.pop_front();
            log::debug!("dropping unclaimed interrupt for request {dropped:?}");
        }

        self.ids.push_back(id);
    }

    fn take(&mut self, id: u64) -> bool {
        if let Some(pos) = self.ids.iter().position(|&pending| pending == id) {
            self.ids.remove(pos);
            true
        } else {
            false
        }
    }
}

impl<F, U: Unmount> std::ops::Deref for Server<F, U> {
    type Target = ServerInner<U::Conn>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

type Token<U> = TokenGuard<<<U as Unmount>::Conn as ConnectionMeta>::ReqToken>;

impl<F, U> Server<F, U>
where
    F: Fs<U::Conn>,
    U: Unmount,
{
    pub(crate) async fn new<B>(h: Handle<B, U>) -> std::io::Result<AsyncRc<Self>>
    where
        B: BindFs<U::Conn, BoundFs = F>,
    {
        let conn = h.conn.bind().await?;

        // Ensure the connection supports at least sending an error (response header only)
        //
        // This is already checked before initialization, but technically the
        // bound connection can return a different value.
        if conn.max_response_size() < crate::proto::MIN_MSG_SIZE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "connection max response size too small",
            ));
        }

        let inner = AsyncRc::new(ServerInner {
            id: h.id,
            access: h.access,
            conn,
            minor_ver: h.minor_ver,
            flags: h.flags,
            caps: h.config.caps(),
            ignore_interrupts: h.config.ignore_interrupts(),
            buf_pool: BufPool::new(),
            // A write request is a header plus up to `max_write` bytes.
            buf_size: h.config.max_write() + BUF_HEADER_SIZE,
            open_reqs: RefCell::new(HashMap::new()),
            early_interrupts: RefCell::new(EarlyInterrupts::default()),
            cancel_tokens: RefCell::new(Vec::new()),
            replies: ReplyState {
                pending: RefCell::new(HashMap::new()),
                next_id: Cell::new(0),
            },
            mesh_rx: h.mesh_rx,
            mesh_tx: h.mesh_tx,
        });

        let fs =
            h.fs.bind(crate::context::Context::new(inner.clone()))
                .await?;

        Ok(AsyncRc::new(Self {
            inner,
            fs,
            once: h.once,
        }))
    }

    pub(crate) async fn serve_requests(this: &AsyncRc<Self>) {
        loop {
            match Self::serve_one(this).await {
                ControlFlow::Break(true) => {
                    this.broadcast(Message::Shutdown).await;
                    return;
                }
                ControlFlow::Break(false) => return,
                ControlFlow::Continue(()) => {}
            }
        }
    }

    pub(crate) async fn unmount(this: AsyncRc<Self>) -> Result<()> {
        let Self {
            inner, fs, once, ..
        } = this.unwrap().await;

        fs.unmount(crate::context::Context::new(inner.clone()))
            .await;

        let inner = inner.unwrap().await;

        if let Some(once) = once {
            once.unmount
                .unmount(Conn::Bound(inner.conn), &once.path, &once.opts)
                .await?;
        }

        Ok(())
    }

    async fn serve_one(this: &AsyncRc<Self>) -> ControlFlow<bool> {
        match crate::select_biased(
            Self::serve_messages(this),
            this.conn
                .recv_request(this.buf_pool.checkout_with_capacity(this.buf_size)),
        )
        .await
        {
            either::Left(()) => ControlFlow::Break(false),
            either::Right(BufResult(Ok((len, token)), mut buf)) => {
                unsafe {
                    buf.set_len(len);
                }

                Self::handle_req(this, TokenGuard::new(token), buf).await
            }
            either::Right(BufResult(Err(err), _)) => Self::handle_error(err),
        }
    }

    async fn serve_messages(this: &AsyncRc<Self>) {
        loop {
            match this.mesh_rx.recv().await {
                Ok(Message::Shutdown) | Err(_) => return,
                Ok(Message::Interrupt(id)) => {
                    Self::interrupt(this, id);
                }
                Ok(Message::Reply(id, reply)) => {
                    let reply = reply.map(|reply| {
                        drop(this.inner.buf_pool.checkout().steal());
                        reply.bind(&this.inner.buf_pool)
                    });
                    Self::handle_reply(this, id, reply);
                }
            }
        }
    }

    /// Cancels the request with the given id, returning whether this worker was
    /// running it.
    ///
    /// When it wasn't, the id is remembered so that the request cancels itself
    /// as soon as it is registered.
    fn interrupt(this: &AsyncRc<Self>, id: u64) -> bool {
        if let Some(token) = this.open_reqs.borrow_mut().remove(&id) {
            token.cancel();
            true
        } else {
            this.early_interrupts.borrow_mut().record(id);
            false
        }
    }

    fn handle_reply(&self, id: u32, reply: crate::Result<NotifyReply>) {
        if let Some(tx) = self.inner.replies.pending.borrow_mut().remove(&id) {
            let _ = tx.send(reply);
        }
    }

    fn handle_error(err: std::io::Error) -> ControlFlow<bool> {
        match err.raw_os_error() {
            // Interrupted, or the request was cancelled before it could be read.
            Some(nix::libc::EINTR | nix::libc::EAGAIN | nix::libc::ENOENT) => {
                ControlFlow::Continue(())
            }
            // ENODEV means the filesystem was unmounted or the connection was
            // aborted via /sys/fs/fuse/connections. Anything else is unexpected;
            // either way this worker stops rather than spinning on the error.
            _ => {
                log::error!("FUSE device connection broken: {err}");
                ControlFlow::Break(true) // abort all workers
            }
        }
    }

    async fn handle_req(this: &AsyncRc<Self>, token: Token<U>, buf: Buf) -> ControlFlow<bool> {
        if buf.len() < std::mem::size_of::<crate::proto::request::RawHeader>() {
            this.discard_token(token).await;

            if buf.is_empty() {
                return ControlFlow::Break(false);
            }

            log::error!(
                "worker {} received incomplete request from kernel: {} bytes received",
                this.inner.id,
                buf.len()
            );
            return ControlFlow::Continue(());
        }

        let crate::proto::request::RawHeader {
            unique: id,
            opcode: op,
            uid,
            ..
        } = unsafe { std::ptr::read(buf.as_ptr().cast()) };

        // `allow_root` asks the kernel for `allow_other` and leaves the
        // narrowing to here, so this is the only thing keeping the mount from
        // being readable by everyone. Refused before the body is decoded: a
        // caller who may not ask has no claim on the work of understanding the
        // question.
        if !this.access.allows(uid, op) {
            log::debug!(
                "worker {} refused request from uid {uid}: id={id}, opcode={op}",
                this.inner.id,
            );

            let this = this.clone();
            compio::runtime::spawn(async move {
                let _ = this
                    .conn
                    .send_response_buf(token.into_inner(), Error::EACCES.into_reply(id))
                    .await;
            })
            .detach();

            return ControlFlow::Continue(());
        }

        let full_req = match AnyRequest::decode(buf, this.minor_ver, this.flags) {
            Ok(req) => req,
            Err(err) => {
                log::error!(
                    "worker {} received invalid request from kernel: id={}, opcode={}",
                    this.inner.id,
                    id,
                    op,
                );

                match op {
                    Opcode::FORGET | Opcode::BATCH_FORGET | Opcode::MONITOR => {
                        this.discard_token(token).await
                    }
                    Opcode::NOTIFY_REPLY => {
                        this.discard_token(token).await;
                        let worker = (id >> 32) as usize;
                        let reply_id = id as u32;
                        if worker == this.inner.id {
                            Self::handle_reply(this, reply_id, Err(err));
                        } else {
                            let this = this.clone();
                            compio::runtime::spawn(async move {
                                // NOTIFY_REPLY takes no reply, so a bad worker
                                // index can only be logged.
                                let Some(tx) = this.mesh_tx.get(worker) else {
                                    log::error!(
                                        "received NOTIFY_REPLY for unknown worker {worker}"
                                    );
                                    return;
                                };
                                let _ = tx.send(Message::Reply(reply_id, Err(err))).await;
                            })
                            .detach();
                        }
                    }
                    _ => {
                        let this = this.clone();
                        compio::runtime::spawn(async move {
                            let _ = this
                                .conn
                                .send_response_buf(token.into_inner(), err.into_reply(id))
                                .await;
                        })
                        .detach();
                    }
                }

                return ControlFlow::Continue(());
            }
        };

        log::trace!("worker {} received request: {:?}", this.inner.id, full_req);

        let AnyRequest { req, body } = full_req;

        match body {
            Body::Interrupt(intr) => {
                this.discard_token(token).await;
                if !this.ignore_interrupts {
                    let id = intr.id();

                    // The request may belong to another worker, or may not have
                    // been decoded yet, so relay the interrupt when this worker
                    // doesn't hold it.
                    if !Self::interrupt(this, id) {
                        Self::broadcast_detached(this, Message::Interrupt(id));
                    }
                }
            }
            Body::NotifyReply(notify) => {
                this.discard_token(token).await;
                let id = req.id();
                let worker = (id >> 32) as usize;
                let reply_id = id as u32;
                if worker == this.inner.id {
                    Self::handle_reply(this, reply_id, Ok(notify));
                } else {
                    let this = this.clone();
                    compio::runtime::spawn(async move {
                        let Some(tx) = this.mesh_tx.get(worker) else {
                            return;
                        };
                        let _ = tx.send(Message::Reply(reply_id, Ok(notify.share()))).await;
                    })
                    .detach();
                }
            }
            // DESTROY is synchronous: the kernel waits for this reply before the
            // unmount completes.
            Body::Destroy(_) => {
                log::info!("kernel initiated unmount");
                this.send(token, id, ()).await;
                return ControlFlow::Break(true);
            }
            body => {
                let cancel = this
                    .cancel_tokens
                    .borrow_mut()
                    .pop()
                    .unwrap_or_else(CancelToken::new);
                if !this.ignore_interrupts {
                    // An interrupt for this request may have arrived before the
                    // request itself.
                    if this.inner.early_interrupts.borrow_mut().take(id) {
                        cancel.cancel();
                    }

                    this.inner.open_reqs.borrow_mut().insert(id, cancel.clone());
                }
                compio::runtime::spawn(Self::handle_fs_op(this.clone(), cancel, token, req, body))
                    .detach();
            }
        }

        ControlFlow::Continue(())
    }

    async fn handle_fs_op(
        this: AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: crate::types::Request,
        body: Body,
    ) {
        struct Guard<'a, C> {
            server: &'a ServerInner<C>,
            id: u64,
            cancel: CancelToken,
        }

        impl<C> Drop for Guard<'_, C> {
            fn drop(&mut self) {
                drop(self.server.open_reqs.borrow_mut().remove(&self.id));
                if self.cancel.try_reset() {
                    self.server
                        .cancel_tokens
                        .borrow_mut()
                        .push(self.cancel.clone());
                }
            }
        }

        let _guard = Guard {
            server: &this.inner,
            id: req.id,
            cancel: cancel.clone(),
        };

        match body {
            Body::Lookup(body) => Self::lookup(&this, cancel, token, req, body).await,
            Body::Forget(body) => Self::forget(&this, cancel, token, req, body).await,
            Body::GetAttr(body) => Self::getattr(&this, cancel, token, req, body).await,
            Body::SetAttr(body) => Self::setattr(&this, cancel, token, req, body).await,
            Body::ReadLink(body) => Self::readlink(&this, cancel, token, req, body).await,
            Body::Symlink(body) => Self::symlink(&this, cancel, token, req, body).await,
            Body::MkNod(body) => Self::mknod(&this, cancel, token, req, body).await,
            Body::MkDir(body) => Self::mkdir(&this, cancel, token, req, body).await,
            Body::Unlink(body) => Self::unlink(&this, cancel, token, req, body).await,
            Body::RmDir(body) => Self::rmdir(&this, cancel, token, req, body).await,
            Body::Rename(body) => Self::rename(&this, cancel, token, req, body).await,
            Body::Link(body) => Self::link(&this, cancel, token, req, body).await,
            Body::Open(body) => Self::open(&this, cancel, token, req, body).await,
            Body::Read(body) => Self::read(&this, cancel, token, req, body).await,
            Body::Write(body) => Self::write(&this, cancel, token, req, body).await,
            Body::StatFs(body) => Self::statfs(&this, cancel, token, req, body).await,
            Body::Release(body) => Self::release(&this, cancel, token, req, body).await,
            Body::Fsync(body) => Self::fsync(&this, cancel, token, req, body).await,
            Body::SetXattr(body) => Self::setxattr(&this, cancel, token, req, body).await,
            Body::GetXattr(body) => Self::getxattr(&this, cancel, token, req, body).await,
            Body::ListXattr(body) => Self::listxattr(&this, cancel, token, req, body).await,
            Body::RemoveXattr(body) => Self::removexattr(&this, cancel, token, req, body).await,
            Body::Flush(body) => Self::flush(&this, cancel, token, req, body).await,
            Body::OpenDir(body) => Self::opendir(&this, cancel, token, req, body).await,
            Body::ReadDir(body) => Self::readdir(&this, cancel, token, req, body).await,
            Body::ReleaseDir(body) => Self::releasedir(&this, cancel, token, req, body).await,
            Body::FsyncDir(body) => Self::fsyncdir(&this, cancel, token, req, body).await,
            Body::GetLk(body) => Self::getlk(&this, cancel, token, req, body).await,
            Body::SetLk(body) => Self::setlk(&this, cancel, token, req, body).await,
            Body::SetLkW(body) => Self::setlkw(&this, cancel, token, req, body).await,
            Body::Access(body) => Self::access(&this, cancel, token, req, body).await,
            Body::Create(body) => Self::create(&this, cancel, token, req, body).await,
            Body::Bmap(body) => Self::bmap(&this, cancel, token, req, body).await,
            Body::Ioctl(body) => Self::ioctl(&this, cancel, token, req, body).await,
            Body::Poll(body) => Self::poll(&this, cancel, token, req, body).await,
            Body::BatchForget(body) => Self::batchforget(&this, cancel, token, req, body).await,
            Body::Fallocate(body) => Self::fallocate(&this, cancel, token, req, body).await,
            Body::ReadDirPlus(body) => Self::readdirplus(&this, cancel, token, req, body).await,
            Body::Lseek(body) => Self::lseek(&this, cancel, token, req, body).await,
            Body::CopyFileRange(body) => {
                Self::copy_file_range(&this, cancel, token, req, body).await
            }
            Body::SyncFs(body) => Self::syncfs(&this, cancel, token, req, body).await,
            Body::TmpFile(body) => Self::tmp_file(&this, cancel, token, req, body).await,
            Body::StatX(body) => Self::statx(&this, cancel, token, req, body).await,
            Body::CopyFileRange64(body) => {
                Self::copy_file_range64(&this, cancel, token, req, body).await
            }
            Body::SetVolName(body) => Self::setvolname(&this, cancel, token, req, body).await,
            Body::GetXtimes(body) => Self::getxtimes(&this, cancel, token, req, body).await,
            Body::Monitor(body) => Self::monitor(&this, cancel, token, req, body).await,

            // `handle_req` answers these itself and never dispatches them. Named
            // rather than caught by a wildcard so that a new `Body` variant is a
            // compile error here instead of a request the server quietly drops.
            Body::Interrupt(_) | Body::Destroy(_) | Body::NotifyReply(_) => {
                log::error!(
                    "worker {} dispatched request {} which is handled before dispatch",
                    this.inner.id,
                    req.id,
                );
                this.discard_token(token).await;
            }
        }
    }

    async fn lookup(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::Lookup,
    ) {
        let id = req.id;
        let req = req::LookupReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.lookup(req).await).await;
    }

    async fn forget(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::Forget,
    ) {
        let req = req::ForgetReq::from_single(Req::new(this, cancel, req), body);
        this.fs.forget(req).await;
        this.discard_token(token).await;
    }

    async fn getattr(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::GetAttr,
    ) {
        let id = req.id;
        let req = req::GetAttrsReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.get_attrs(req).await).await;
    }

    async fn setattr(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::SetAttr,
    ) {
        let id = req.id;
        let req = req::SetAttrsReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.set_attrs(req).await).await;
    }

    async fn readlink(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::ReadLink,
    ) {
        let id = req.id;
        let req = req::ReadLinkReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.read_link(req).await.map(Data::new))
            .await;
    }

    async fn symlink(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::Symlink,
    ) {
        let id = req.id;
        let req = req::SymlinkReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.symlink(req).await).await;
    }

    async fn mknod(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::MkNod,
    ) {
        let id = req.id;
        let req = req::MakeNodeReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.make_node(req).await).await;
    }

    async fn mkdir(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::MkDir,
    ) {
        let id = req.id;
        let req = req::MakeDirReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.make_dir(req).await).await;
    }

    async fn unlink(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::Unlink,
    ) {
        let id = req.id;
        let req = req::UnlinkNodeReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.unlink_node(req).await).await;
    }

    async fn rmdir(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::RmDir,
    ) {
        let id = req.id;
        let req = req::RemoveDirReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.remove_dir(req).await).await;
    }

    async fn rename(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::Rename,
    ) {
        let id = req.id;
        let mode = body.rename_mode();

        if !this.caps.contains(rename_caps(mode)) {
            this.send(token, id, unsupported_rename()).await;
            return;
        }

        let req = req::RenameReq::new(Req::new(this, cancel, req), body);

        // Replying ENOSYS to a flagged rename makes Linux stop sending RENAME2
        // for the whole mount, which would disable every mode at once.
        let resp = match this.fs.rename(req).await {
            Err(err) if err == Error::ENOSYS && mode != RenameMode::Replace => {
                Err(unsupported_rename())
            }
            resp => resp,
        };

        this.send(token, id, resp).await;
    }

    async fn link(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::Link,
    ) {
        let id = req.id;
        let req = req::LinkReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.link(req).await).await;
    }

    async fn open(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::Open,
    ) {
        let id = req.id;
        let req = req::OpenReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.open(req).await).await;
    }

    async fn read(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::Read,
    ) {
        let id = req.id;
        let len = body.len();
        let req = req::ReadReq::new(Req::new(this, cancel, req), body);

        let resp = match this.fs.read(req).await {
            Ok(data) => {
                let data = data.into_io_buf();
                let read = data.total_len();

                // The kernel fails a read whose reply overruns the size it asked
                // for, without saying why, so name the filesystem's mistake.
                if read > len {
                    log::error!("read reply is {read} bytes, but only {len} were requested");
                    Err(Error::EIO)
                } else {
                    Ok(Data::new(data))
                }
            }
            Err(err) => Err(err),
        };

        this.send(token, id, resp).await;
    }

    async fn write(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::Write,
    ) {
        let id = req.id;
        let req = req::WriteReq::new(Req::new(this, cancel, req), body);
        let resp = this.fs.write(req).await.map(Write::new);
        this.send(token, id, resp).await;
    }

    async fn statfs(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::StatFs,
    ) {
        let id = req.id;
        let req = req::StatFsReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.statfs(req).await).await;
    }

    async fn release(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::Release,
    ) {
        let id = req.id;
        let req = req::CloseReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.close(req).await).await;
    }

    async fn fsync(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::Fsync,
    ) {
        let id = req.id;
        let req = req::FsyncReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.fsync(req).await).await;
    }

    async fn setxattr(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::SetXattr,
    ) {
        let id = req.id;
        let req = req::SetXattrReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.set_xattr(req).await).await;
    }

    async fn getxattr(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::GetXattr,
    ) {
        let id = req.id;
        let len = body.len();
        if len == 0 {
            let req = req::GetXattrLenReq::new(Req::new(this, cancel, req), body);
            match this.fs.get_xattr_len(req).await {
                Ok(len) => match u32::try_from(len) {
                    Ok(len) => this.send(token, id, XattrLen::new(len)).await,
                    Err(_) => this.send(token, id, Error::E2BIG).await,
                },
                Err(err) => this.send(token, id, err).await,
            }
        } else {
            let req = req::GetXattrReq::new(Req::new(this, cancel, req), body);
            let data = match this.fs.get_xattr(req).await {
                Ok(data) => data.into_io_buf(),
                Err(err) => {
                    this.send(token, id, err).await;
                    return;
                }
            };

            if data.total_len() > len {
                this.send(token, id, Error::ERANGE).await;
                return;
            }

            this.send(token, id, Data::new(data)).await;
        }
    }

    async fn listxattr(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::ListXattr,
    ) {
        let id = req.id;
        let len = body.len();
        if len == 0 {
            let req = req::XattrKeysLenReq::new(Req::new(this, cancel, req), body);
            let len = match this.fs.xattr_keys_len(req).await {
                Ok(buf) => buf.len,
                Err(err) => {
                    this.send(token, id, err).await;
                    return;
                }
            };
            this.send(token, id, XattrLen::new(len)).await;
        } else {
            let req = req::XattrKeysReq::new(Req::new(this, cancel, req), body);
            this.send(
                token,
                id,
                this.fs.xattr_keys(req).await.map(XattrKeyBuf::into_data),
            )
            .await;
        }
    }

    async fn removexattr(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::RemoveXattr,
    ) {
        let id = req.id;
        let req = req::RemoveXattrReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.remove_xattr(req).await).await;
    }

    async fn flush(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::Flush,
    ) {
        let id = req.id;
        let req = req::FlushReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.flush(req).await).await;
    }

    async fn opendir(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::OpenDir,
    ) {
        let id = req.id;
        let req = req::OpenReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.open(req).await).await;
    }

    async fn readdir(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::ReadDir,
    ) {
        let id = req.id;
        let req = req::ReadDirReq::new(Req::new(this, cancel, req), body);
        this.send(
            token,
            id,
            this.fs.read_dir(req).await.map(DirEntryBuf::into_data),
        )
        .await;
    }

    async fn releasedir(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::ReleaseDir,
    ) {
        let id = req.id;
        let req = req::CloseReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.close(req).await).await;
    }

    async fn fsyncdir(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::FsyncDir,
    ) {
        let id = req.id;
        let req = req::FsyncReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.fsync(req).await).await;
    }

    async fn getlk(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::GetLk,
    ) {
        let id = req.id;
        let req = req::TestPosixLockReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.test_posix_lock(req).await)
            .await;
    }

    async fn setlk(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::SetLk,
    ) {
        let id = req.id;
        if body.is_flock() {
            let req = req::FlockReq::new(Req::new(this, cancel, req), body);
            this.send(token, id, this.fs.try_flock(req).await).await;
        } else {
            let req = req::PosixLockReq::new(Req::new(this, cancel, req), body);
            this.send(token, id, this.fs.try_posix_lock(req).await)
                .await;
        }
    }

    async fn setlkw(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::SetLkW,
    ) {
        let id = req.id;
        if body.is_flock() {
            let req = req::FlockReq::new(Req::new(this, cancel, req), body);
            this.send(token, id, this.fs.flock(req).await).await;
        } else {
            let req = req::PosixLockReq::new(Req::new(this, cancel, req), body);
            this.send(token, id, this.fs.posix_lock(req).await).await;
        }
    }

    async fn access(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::Access,
    ) {
        let id = req.id;
        let req = req::AccessReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.access(req).await).await;
    }

    async fn create(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::Create,
    ) {
        let id = req.id;
        let req = req::CreateFileReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.create_file(req).await).await;
    }

    async fn bmap(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::Bmap,
    ) {
        let id = req.id;
        let req = req::MapBlockReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.map_block(req).await.map(Bmap::new))
            .await;
    }

    async fn ioctl(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::Ioctl,
    ) {
        let id = req.id;
        let out_len = body.out_len();
        let cmd = body.command();
        let req = req::IoctlReq::new(Req::new(this, cancel, req), body);
        match this.fs.ioctl(req).await {
            Ok(reply) => {
                let IoctlReply { value, data } = reply;
                let data = data.into_io_buf();
                let len = data.total_len();
                if len > out_len {
                    log::error!("ioctl reply for {cmd:?} is {len} bytes, but out_len is {out_len}");
                    this.send(token, id, Error::EIO).await;
                    return;
                }
                this.send(token, id, IoctlReply { value, data }).await;
            }
            Err(err) => this.send(token, id, err).await,
        }
    }

    async fn poll(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::Poll,
    ) {
        let id = req.id;
        let req = req::PollReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.poll(req).await.map(Poll::new))
            .await;
    }

    async fn batchforget(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::BatchForget,
    ) {
        let req = req::ForgetReq::from_batch(Req::new(this, cancel, req), body);
        this.fs.forget(req).await;
        this.discard_token(token).await;
    }

    async fn fallocate(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::Fallocate,
    ) {
        let id = req.id;

        if !this.caps.contains(FsCaps::FALLOCATE) {
            this.send(token, id, Error::ENOSYS).await;
            return;
        }

        let req = req::FallocateReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.fallocate(req).await).await;
    }

    async fn readdirplus(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::ReadDirPlus,
    ) {
        let id = req.id;
        let req = req::ReadDirPlusReq::new(Req::new(this, cancel, req), body);
        this.send(
            token,
            id,
            this.fs
                .read_dir_plus(req)
                .await
                .map(DirEntryPlusBuf::into_data),
        )
        .await;
    }

    async fn lseek(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::Lseek,
    ) {
        let id = req.id;
        let req = req::LseekReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.lseek(req).await.map(Lseek::new))
            .await;
    }

    /// Serves `FUSE_COPY_FILE_RANGE`, whose reply counts the copied bytes in a
    /// `u32`.
    ///
    /// The requested length is clamped when it is decoded, so a filesystem that
    /// respects it can always report its result.
    async fn copy_file_range(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::CopyFileRange,
    ) {
        let id = req.id;
        let req = req::CopyFileRangeReq::new(Req::new(this, cancel, req), body);
        let resp = match this.fs.copy_file_range(req).await {
            Ok(copied) => match u32::try_from(copied) {
                Ok(copied) => Ok(Write::new(copied as usize)),
                Err(_) => {
                    log::error!(
                        "filesystem copied {copied} bytes, which COPY_FILE_RANGE cannot report"
                    );
                    Err(Error::EIO)
                }
            },
            Err(err) => Err(err),
        };

        this.send(token, id, resp).await;
    }

    async fn syncfs(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::SyncFs,
    ) {
        let id = req.id;
        let req = req::SyncFsReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.syncfs(req).await).await;
    }

    async fn tmp_file(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::TmpFile,
    ) {
        let id = req.id;
        let req = req::TmpFileReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.tmp_file(req).await).await;
    }

    async fn statx(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::StatX,
    ) {
        let id = req.id;
        let req = req::StatXReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.statx(req).await).await;
    }

    async fn copy_file_range64(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::CopyFileRange64,
    ) {
        let id = req.id;
        let req = req::CopyFileRangeReq::new(Req::new(this, cancel, req), body);
        this.send(
            token,
            id,
            this.fs.copy_file_range(req).await.map(CopyFileRange::new),
        )
        .await;
    }

    async fn setvolname(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::SetVolName,
    ) {
        let id = req.id;
        let req = req::SetVolumeNameReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.set_volume_name(req).await)
            .await;
    }

    async fn getxtimes(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::GetXtimes,
    ) {
        let id = req.id;
        let req = req::GetXTimesReq::new(Req::new(this, cancel, req), body);
        this.send(token, id, this.fs.get_xtimes(req).await).await;
    }

    async fn monitor(
        this: &AsyncRc<Self>,
        cancel: CancelToken,
        token: Token<U>,
        req: Request,
        body: request::Monitor,
    ) {
        let req = req::MonitorReq::new(Req::new(this, cancel, req), body);
        this.fs.monitor(req).await;
        this.discard_token(token).await;
    }

    async fn send<T>(&self, token: Token<U>, id: u64, resp: T)
    where
        T: EncodeResp,
        Error: From<T::Error>,
    {
        let cfg = crate::proto::Cfg {
            minor_ver: self.minor_ver,
            flags: self.flags,
        };

        let res = match resp.encode(id, cfg) {
            Ok(buf) => {
                let buf = buf.into_io_buf();
                let response_size = buf.total_len();
                let max_response_size = self.conn.max_response_size();
                if response_size > max_response_size {
                    log::error!(
                        "response for message {} on worker {} is larger than the maximum response size ({}): size={}",
                        id,
                        self.inner.id,
                        max_response_size,
                        response_size,
                    );
                    let Ok(buf) = Error::EIO.encode(id, cfg);
                    self.conn.send_response_buf(token.into_inner(), buf).await
                } else {
                    self.conn.send_response_buf(token.into_inner(), buf).await
                }
            }
            Err(err) => {
                let Ok(buf) = Error::from(err).encode(id, cfg);
                self.conn.send_response_buf(token.into_inner(), buf).await
            }
        };

        if let Err(err) = res {
            log::error!(
                "worker {} failed to send response on FUSE device: {}",
                self.inner.id,
                err
            );
        }
    }

    async fn discard_token(&self, token: Token<U>) {
        self.conn.discard(token.into_inner()).await;
    }

    async fn broadcast(&self, msg: Message) {
        for tx in self
            .mesh_tx
            .iter()
            .enumerate()
            .filter_map(|(idx, tx)| (idx != self.id).then_some(tx))
        {
            let _ = tx.send(msg.clone()).await;
        }
    }

    fn broadcast_detached(this: &AsyncRc<Self>, msg: Message) {
        let this = this.clone();
        compio::runtime::spawn(async move { this.broadcast(msg).await }).detach();
    }
}

/// The capabilities a rename mode requires the filesystem to have enabled.
fn rename_caps(mode: RenameMode) -> FsCaps {
    match mode {
        RenameMode::Replace => FsCaps::empty(),
        RenameMode::NoReplace => FsCaps::RENAME_NOREPLACE,
        RenameMode::Exchange => FsCaps::RENAME_EXCHANGE,
        RenameMode::Whiteout => FsCaps::RENAME_WHITEOUT,
        RenameMode::WhiteoutNoReplace => FsCaps::RENAME_WHITEOUT.union(FsCaps::RENAME_NOREPLACE),
        RenameMode::ExchangeData => FsCaps::EXCHANGE_DATA,
    }
}

/// The errno for a rename mode the filesystem didn't enable.
///
/// Linux treats ENOSYS from RENAME2 as "no flagged renames at all", so an
/// unsupported mode gets EINVAL instead, like a kernel that doesn't know the
/// flag. macOS reports renamex_np(2) failures as ENOTSUP.
const fn unsupported_rename() -> Error {
    if cfg!(target_os = "macos") {
        Error::ENOTSUP
    } else {
        Error::EINVAL
    }
}

impl ReplyState {
    pub fn channel(&self) -> (u32, unsync::oneshot::Receiver<crate::Result<NotifyReply>>) {
        let id = self.next_id.get();
        self.next_id.set(id.wrapping_add(1));

        let (tx, rx) = unsync::oneshot::channel();

        self.pending.borrow_mut().insert(id, tx);

        (id, rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_early_interrupt_is_claimed_once() {
        let mut early = EarlyInterrupts::default();

        early.record(7);

        assert!(early.take(7));
        assert!(!early.take(7));
    }

    #[test]
    fn unrelated_requests_dont_claim_an_interrupt() {
        let mut early = EarlyInterrupts::default();

        early.record(7);

        assert!(!early.take(8));
        assert!(early.take(7));
    }

    #[test]
    fn repeating_an_interrupt_keeps_one_entry() {
        let mut early = EarlyInterrupts::default();

        early.record(7);
        early.record(7);

        assert!(early.take(7));
        assert!(!early.take(7));
    }

    #[test]
    fn interrupts_are_claimed_in_any_order() {
        let mut early = EarlyInterrupts::default();

        early.record(1);
        early.record(2);
        early.record(3);

        assert!(early.take(2));
        assert!(early.take(1));
        assert!(early.take(3));
        assert!(early.ids.is_empty());
    }

    #[test]
    fn unclaimed_interrupts_are_bounded() {
        let mut early = EarlyInterrupts::default();

        for id in 0..(MAX_EARLY_INTERRUPTS as u64 * 4) {
            early.record(id);
        }

        assert_eq!(early.ids.len(), MAX_EARLY_INTERRUPTS);
    }

    #[test]
    fn the_oldest_unclaimed_interrupt_is_dropped_first() {
        let mut early = EarlyInterrupts::default();

        for id in 0..=(MAX_EARLY_INTERRUPTS as u64) {
            early.record(id);
        }

        assert!(!early.take(0));
        assert!(early.take(1));
        assert!(early.take(MAX_EARLY_INTERRUPTS as u64));
    }
}
