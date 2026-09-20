use crate::Error;
use crate::async_rc::AsyncRc;
use crate::buf::{Buf, BufPool, IntoIoBuf};
use crate::dev_fuse::FuseChannel;
use crate::fs::req::{self, Req};
use crate::fs::{BindFs, Fs};
use crate::handle::{Handle, Once};
use crate::mount::Unmount;
use crate::proto::request::{self, AnyRequest, Body, NotifyReply, SharedNotifyReply};
use crate::proto::response::{Bmap, CopyFileRange, Data, EncodeResp, Lseek, Poll, Write, XattrLen};
use crate::req::{DirEntryBuf, DirEntryPlusBuf, XattrKeyBuf};
use crate::types::{FsCaps, RenameMode, ReplyInitFlags, Request};

use compio::runtime::JoinHandle;
use compio::BufResult;
use crossfire::{AsyncRx, MAsyncTx, mpsc::Array};
use futures_util::StreamExt;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io::Result;
use std::ops::ControlFlow;
use std::os::fd::AsFd;
use std::rc::Rc;
use std::sync::Arc;

pub const MAX_WRITE_SIZE: usize = 16 * 1024 * 1024;

/// `FUSE_BUFFER_HEADER_SIZE`: the room a request needs on top of `max_write`.
pub const BUF_HEADER_SIZE: usize = 4096;

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
    pub caps: FsCaps,
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
                caps: h.config.caps(),
                buf_pool: BufPool::new(),
                // A write request is a header plus up to `max_write` bytes.
                buf_size: h.config.max_write() as usize + BUF_HEADER_SIZE,
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
        loop {
            match Self::serve_one(this).await {
                ControlFlow::Break(true) => {
                    for tx in this
                        .inner
                        .mesh_tx
                        .iter()
                        .enumerate()
                        .filter_map(|(id, tx)| (id != this.inner.id).then_some(tx))
                    {
                        let _ = tx.send(Message::Shutdown).await;
                    }

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

        fs.unmount().await;

        if let Some(once) = once {
            once.unmount
                .unmount(inner.dev.as_fd(), &once.path, &once.opts)
                .await?;
        }

        Ok(())
    }

    async fn serve_one(this: &AsyncRc<Self>) -> ControlFlow<bool> {
        match select_biased(
            Self::serve_messages(this),
            this.dev
                .read(this.buf_pool.checkout_with_capacity(this.buf_size)),
        )
        .await
        {
            either::Left(()) => ControlFlow::Break(false),
            either::Right(BufResult(Ok(len), mut buf)) => {
                unsafe {
                    buf.set_len(len);
                }

                Self::handle_req(this, buf).await
            }
            either::Right(BufResult(Err(err), _)) => Self::handle_error(err),
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
                    drop(this.inner.buf_pool.checkout().steal());
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

    async fn handle_req(this: &AsyncRc<Self>, buf: Buf) -> ControlFlow<bool> {
        if buf.len() < std::mem::size_of::<crate::proto::request::RawHeader>() {
            log::error!("worker {} received incomplete request from kernel: {} bytes received", this.inner.id, buf.len());
            return ControlFlow::Continue(());
        }

        let id = unsafe { (*(buf.as_ptr() as *const crate::proto::request::RawHeader)).unique };

        let full_req = match AnyRequest::decode(buf, this.minor_ver, this.flags) {
            Ok(req) => req,
            Err(err) => {
                log::error!("worker {} received invalid request from kernel: id={}", this.inner.id, id);

                let this = this.clone();
                compio::runtime::spawn(async move {
                    let _ = this.dev.write_buf(err.into_reply(id)).await;
                })
                .detach();
                return ControlFlow::Continue(());
            }
        };

        log::trace!("worker {} received request: {:?}", this.inner.id, full_req);

        let AnyRequest { req, body } = full_req;

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
                let reply_id = id as u32;
                if worker == this.inner.id {
                    Self::handle_reply(this, reply_id, notify);
                } else {
                    let this = this.clone();
                    compio::runtime::spawn(async move {
                        let Some(tx) = this.mesh_tx.get(worker) else {
                            this.send(id, Error::EINVAL).await;
                            return;
                        };
                        let _ = tx.send(Message::Reply(reply_id, notify.share())).await;
                    })
                    .detach();
                }
            }
            // DESTROY is synchronous: the kernel waits for this reply before the
            // unmount completes.
            Body::Destroy(_) => {
                log::info!("kernel initiated unmount");
                this.send(id, ()).await;
                return ControlFlow::Break(true);
            }
            body => {
                let task = compio::runtime::spawn(Self::handle_fs_op(this.clone(), req, body));
                this.inner.open_reqs.borrow_mut().insert(id, task);
            }
        }

        ControlFlow::Continue(())
    }

    async fn handle_fs_op(this: AsyncRc<Self>, req: crate::types::Request, body: Body) {
        struct Guard<'a> {
            server: &'a ServerInner,
            id: u64,
        }

        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                self.server.open_reqs.borrow_mut().remove(&self.id);
            }
        }

        let _guard = Guard {
            server: &this.inner,
            id: req.id,
        };

        match body {
            Body::Lookup(body) => Self::lookup(&this, req, body).await,
            Body::Forget(body) => Self::forget(&this, req, body).await,
            Body::GetAttr(body) => Self::getattr(&this, req, body).await,
            Body::SetAttr(body) => Self::setattr(&this, req, body).await,
            Body::ReadLink(body) => Self::readlink(&this, req, body).await,
            Body::Symlink(body) => Self::symlink(&this, req, body).await,
            Body::MkNod(body) => Self::mknod(&this, req, body).await,
            Body::MkDir(body) => Self::mkdir(&this, req, body).await,
            Body::Unlink(body) => Self::unlink(&this, req, body).await,
            Body::RmDir(body) => Self::rmdir(&this, req, body).await,
            Body::Rename(body) => Self::rename(&this, req, body).await,
            Body::Link(body) => Self::link(&this, req, body).await,
            Body::Open(body) => Self::open(&this, req, body).await,
            Body::Read(body) => Self::read(&this, req, body).await,
            Body::Write(body) => Self::write(&this, req, body).await,
            Body::StatFs(body) => Self::statfs(&this, req, body).await,
            Body::Release(body) => Self::release(&this, req, body).await,
            Body::Fsync(body) => Self::fsync(&this, req, body).await,
            Body::SetXattr(body) => Self::setxattr(&this, req, body).await,
            Body::GetXattr(body) => Self::getxattr(&this, req, body).await,
            Body::ListXattr(body) => Self::listxattr(&this, req, body).await,
            Body::RemoveXattr(body) => Self::removexattr(&this, req, body).await,
            Body::Flush(body) => Self::flush(&this, req, body).await,
            Body::OpenDir(body) => Self::opendir(&this, req, body).await,
            Body::ReadDir(body) => Self::readdir(&this, req, body).await,
            Body::ReleaseDir(body) => Self::releasedir(&this, req, body).await,
            Body::FsyncDir(body) => Self::fsyncdir(&this, req, body).await,
            Body::GetLk(body) => Self::getlk(&this, req, body).await,
            Body::SetLk(body) => Self::setlk(&this, req, body).await,
            Body::SetLkW(body) => Self::setlkw(&this, req, body).await,
            Body::Access(body) => Self::access(&this, req, body).await,
            Body::Create(body) => Self::create(&this, req, body).await,
            Body::Bmap(body) => Self::bmap(&this, req, body).await,
            Body::Ioctl(body) => Self::ioctl(&this, req, body).await,
            Body::Poll(body) => Self::poll(&this, req, body).await,
            Body::BatchForget(body) => Self::batchforget(&this, req, body).await,
            Body::Fallocate(body) => Self::fallocate(&this, req, body).await,
            Body::ReadDirPlus(body) => Self::readdirplus(&this, req, body).await,
            Body::Lseek(body) => Self::lseek(&this, req, body).await,
            Body::CopyFileRange(body) => Self::copy_file_range(&this, req, body).await,
            Body::SyncFs(body) => Self::syncfs(&this, req, body).await,
            Body::TmpFile(body) => Self::tmpfile(&this, req, body).await,
            Body::StatX(body) => Self::statx(&this, req, body).await,
            Body::CopyFileRange64(body) => Self::copy_file_range64(&this, req, body).await,
            #[cfg(target_os = "macos")]
            Body::SetVolName(body) => Self::setvolname(&this, req, body).await,
            #[cfg(target_os = "macos")]
            Body::GetXtimes(body) => Self::getxtimes(&this, req, body).await,
            _ => {}
        }
    }

    async fn lookup(this: &AsyncRc<Self>, req: Request, body: request::Lookup) {
        let id = req.id;
        let req = req::LookupReq::new(Req::new(this, req), body);
        this.send(id, this.fs.lookup(req).await).await;
    }

    async fn forget(this: &AsyncRc<Self>, req: Request, body: request::Forget) {
        let req = req::ForgetReq::from_single(Req::new(this, req), body);
        this.fs.forget(req).await;
    }

    async fn getattr(this: &AsyncRc<Self>, req: Request, body: request::GetAttr) {
        let id = req.id;
        let req = req::GetAttrsReq::new(Req::new(this, req), body);
        this.send(id, this.fs.get_attrs(req).await).await;
    }

    async fn setattr(this: &AsyncRc<Self>, req: Request, body: request::SetAttr) {
        let id = req.id;
        let req = req::SetAttrsReq::new(Req::new(this, req), body);
        this.send(id, this.fs.set_attrs(req).await).await;
    }

    async fn readlink(this: &AsyncRc<Self>, req: Request, body: request::ReadLink) {
        let id = req.id;
        let req = req::ReadLinkReq::new(Req::new(this, req), body);
        this.send(id, this.fs.read_link(req).await.map(Data::new))
            .await;
    }

    async fn symlink(this: &AsyncRc<Self>, req: Request, body: request::Symlink) {
        let id = req.id;
        let req = req::SymlinkReq::new(Req::new(this, req), body);
        this.send(id, this.fs.symlink(req).await).await;
    }

    async fn mknod(this: &AsyncRc<Self>, req: Request, body: request::MkNod) {
        let id = req.id;
        let req = req::MakeNodeReq::new(Req::new(this, req), body);
        this.send(id, this.fs.make_node(req).await).await;
    }

    async fn mkdir(this: &AsyncRc<Self>, req: Request, body: request::MkDir) {
        let id = req.id;
        let req = req::MakeDirReq::new(Req::new(this, req), body);
        this.send(id, this.fs.make_dir(req).await).await;
    }

    async fn unlink(this: &AsyncRc<Self>, req: Request, body: request::Unlink) {
        let id = req.id;
        let req = req::UnlinkNodeReq::new(Req::new(this, req), body);
        this.send(id, this.fs.unlink_node(req).await).await;
    }

    async fn rmdir(this: &AsyncRc<Self>, req: Request, body: request::RmDir) {
        let id = req.id;
        let req = req::RemoveDirReq::new(Req::new(this, req), body);
        this.send(id, this.fs.remove_dir(req).await).await;
    }

    async fn rename(this: &AsyncRc<Self>, req: Request, body: request::Rename) {
        let id = req.id;
        let mode = body.rename_mode();

        if !this.caps.contains(rename_caps(mode)) {
            this.send(id, unsupported_rename()).await;
            return;
        }

        let req = req::RenameReq::new(Req::new(this, req), body);

        // Replying ENOSYS to a flagged rename makes Linux stop sending RENAME2
        // for the whole mount, which would disable every mode at once.
        let resp = match this.fs.rename(req).await {
            Err(err) if err == Error::ENOSYS && mode != RenameMode::Replace => {
                Err(unsupported_rename())
            }
            resp => resp,
        };

        this.send(id, resp).await;
    }

    async fn link(this: &AsyncRc<Self>, req: Request, body: request::Link) {
        let id = req.id;
        let req = req::LinkReq::new(Req::new(this, req), body);
        this.send(id, this.fs.link(req).await).await;
    }

    async fn open(this: &AsyncRc<Self>, req: Request, body: request::Open) {
        let id = req.id;
        let req = req::OpenReq::new(Req::new(this, req), body);
        this.send(id, this.fs.open(req).await).await;
    }

    async fn read(this: &AsyncRc<Self>, req: Request, body: request::Read) {
        let id = req.id;
        let req = req::ReadReq::new(Req::new(this, req), body);
        let data = this.fs.read(req).await.map(Data::new);
        this.send(id, data).await;
    }

    async fn write(this: &AsyncRc<Self>, req: Request, body: request::Write) {
        let id = req.id;
        let req = req::WriteReq::new(Req::new(this, req), body);
        let resp = this.fs.write(req).await.map(Write::new);
        this.send(id, resp).await;
    }

    async fn statfs(this: &AsyncRc<Self>, req: Request, body: request::StatFs) {
        let id = req.id;
        let req = req::StatFsReq::new(Req::new(this, req), body);
        this.send(id, this.fs.statfs(req).await).await;
    }

    async fn release(this: &AsyncRc<Self>, req: Request, body: request::Release) {
        let id = req.id;
        let req = req::CloseReq::new(Req::new(this, req), body);
        this.send(id, this.fs.close(req).await).await;
    }

    async fn fsync(this: &AsyncRc<Self>, req: Request, body: request::Fsync) {
        let id = req.id;
        let req = req::FsyncReq::new(Req::new(this, req), body);
        this.send(id, this.fs.fsync(req).await).await;
    }

    async fn setxattr(this: &AsyncRc<Self>, req: Request, body: request::SetXattr) {
        let id = req.id;
        let req = req::SetXattrReq::new(Req::new(this, req), body);
        this.send(id, this.fs.set_xattr(req).await).await;
    }

    async fn getxattr(this: &AsyncRc<Self>, req: Request, body: request::GetXattr) {
        let id = req.id;
        let len = body.len();
        if len == 0 {
            let req = req::GetXattrLenReq::new(Req::new(this, req), body);
            this.send(id, this.fs.get_xattr_len(req).await.map(XattrLen::new))
                .await;
        } else {
            let req = req::GetXattrReq::new(Req::new(this, req), body);
            let data = match this.fs.get_xattr(req).await {
                Ok(data) => data.into_io_buf(),
                Err(err) => {
                    this.send(id, err).await;
                    return;
                }
            };

            if data.total_len() > len {
                this.send(id, Error::ERANGE).await;
                return;
            }

            this.send(id, Data::new(data)).await;
        }
    }

    async fn listxattr(this: &AsyncRc<Self>, req: Request, body: request::ListXattr) {
        let id = req.id;
        let len = body.len();
        if len == 0 {
            let req = req::XattrKeysLenReq::new(Req::new(this, req), body);
            let mut lens = std::pin::pin!(match this.fs.xattr_keys_len(req).await {
                Ok(lens) => lens,
                Err(err) => {
                    this.send(id, err).await;
                    return;
                }
            });

            let mut count = 0usize;
            let mut total = 0usize;
            while let Some(len) = StreamExt::next(&mut lens).await {
                let Some(tmp) = total.checked_add(len) else {
                    this.send(id, Error::E2BIG).await;
                    return;
                };
                total = tmp;
                count += 1;
            }

            // Each key is NUL-terminated.
            let Some(total) = total.checked_add(count) else {
                this.send(id, Error::E2BIG).await;
                return;
            };

            this.send(id, XattrLen::new(total)).await;
        } else {
            let req = req::XattrKeysReq::new(Req::new(this, req), body);
            this.send(
                id,
                this.fs.xattr_keys(req).await.map(XattrKeyBuf::into_data),
            )
            .await;
        }
    }

    async fn removexattr(this: &AsyncRc<Self>, req: Request, body: request::RemoveXattr) {
        let id = req.id;
        let req = req::RemoveXattrReq::new(Req::new(this, req), body);
        this.send(id, this.fs.remove_xattr(req).await).await;
    }

    async fn flush(this: &AsyncRc<Self>, req: Request, body: request::Flush) {
        let id = req.id;
        let req = req::FlushReq::new(Req::new(this, req), body);
        this.send(id, this.fs.flush(req).await).await;
    }

    async fn opendir(this: &AsyncRc<Self>, req: Request, body: request::OpenDir) {
        let id = req.id;
        let req = req::OpenReq::new(Req::new(this, req), body);
        this.send(id, this.fs.open(req).await).await;
    }

    async fn readdir(this: &AsyncRc<Self>, req: Request, body: request::ReadDir) {
        let id = req.id;
        let req = req::ReadDirReq::new(Req::new(this, req), body);
        this.send(id, this.fs.read_dir(req).await.map(DirEntryBuf::into_data))
            .await;
    }

    async fn releasedir(this: &AsyncRc<Self>, req: Request, body: request::ReleaseDir) {
        let id = req.id;
        let req = req::CloseReq::new(Req::new(this, req), body);
        this.send(id, this.fs.close(req).await).await;
    }

    async fn fsyncdir(this: &AsyncRc<Self>, req: Request, body: request::FsyncDir) {
        let id = req.id;
        let req = req::FsyncReq::new(Req::new(this, req), body);
        this.send(id, this.fs.fsync(req).await).await;
    }

    async fn getlk(this: &AsyncRc<Self>, req: Request, body: request::GetLk) {
        let id = req.id;
        let req = req::TestPosixLockReq::new(Req::new(this, req), body);
        this.send(id, this.fs.test_posix_lock(req).await).await;
    }

    async fn setlk(this: &AsyncRc<Self>, req: Request, body: request::SetLk) {
        let id = req.id;
        if body.is_flock() {
            let req = req::FlockReq::new(Req::new(this, req), body);
            this.send(id, this.fs.try_flock(req).await).await;
        } else {
            let req = req::PosixLockReq::new(Req::new(this, req), body);
            this.send(id, this.fs.try_posix_lock(req).await).await;
        }
    }

    async fn setlkw(this: &AsyncRc<Self>, req: Request, body: request::SetLkW) {
        let id = req.id;
        if body.is_flock() {
            let req = req::FlockReq::new(Req::new(this, req), body);
            this.send(id, this.fs.flock(req).await).await;
        } else {
            let req = req::PosixLockReq::new(Req::new(this, req), body);
            this.send(id, this.fs.posix_lock(req).await).await;
        }
    }

    async fn access(this: &AsyncRc<Self>, req: Request, body: request::Access) {
        let id = req.id;
        let req = req::AccessReq::new(Req::new(this, req), body);
        this.send(id, this.fs.access(req).await).await;
    }

    async fn create(this: &AsyncRc<Self>, req: Request, body: request::Create) {
        let id = req.id;
        let req = req::CreateFileReq::new(Req::new(this, req), body);
        this.send(id, this.fs.create_file(req).await).await;
    }

    async fn bmap(this: &AsyncRc<Self>, req: Request, body: request::Bmap) {
        let id = req.id;
        let req = req::MapBlockReq::new(Req::new(this, req), body);
        this.send(id, this.fs.map_block(req).await.map(Bmap::new))
            .await;
    }

    async fn ioctl(this: &AsyncRc<Self>, req: Request, body: request::Ioctl) {
        todo!()
    }

    async fn poll(this: &AsyncRc<Self>, req: Request, body: request::Poll) {
        let id = req.id;
        let req = req::PollReq::new(Req::new(this, req), body);
        this.send(id, this.fs.poll(req).await.map(Poll::new)).await;
    }

    async fn batchforget(this: &AsyncRc<Self>, req: Request, body: request::BatchForget) {
        let req = req::ForgetReq::from_batch(Req::new(this, req), body);
        this.fs.forget(req).await;
    }

    async fn fallocate(this: &AsyncRc<Self>, req: Request, body: request::Fallocate) {
        let id = req.id;

        if !this.caps.contains(FsCaps::FALLOCATE) {
            this.send(id, Error::ENOSYS).await;
            return;
        }

        let req = req::FallocateReq::new(Req::new(this, req), body);
        this.send(id, this.fs.fallocate(req).await).await;
    }

    async fn readdirplus(this: &AsyncRc<Self>, req: Request, body: request::ReadDirPlus) {
        let id = req.id;
        let req = req::ReadDirPlusReq::new(Req::new(this, req), body);
        this.send(
            id,
            this.fs
                .read_dir_plus(req)
                .await
                .map(DirEntryPlusBuf::into_data),
        )
        .await;
    }

    async fn lseek(this: &AsyncRc<Self>, req: Request, body: request::Lseek) {
        let id = req.id;
        let req = req::LseekReq::new(Req::new(this, req), body);
        this.send(id, this.fs.lseek(req).await.map(Lseek::new))
            .await;
    }

    async fn copy_file_range(this: &AsyncRc<Self>, req: Request, body: request::CopyFileRange) {
        let id = req.id;
        let req = req::CopyFileRangeReq::new(Req::new(this, req), body);
        this.send(
            id,
            this.fs.copy_file_range(req).await.map(CopyFileRange::new),
        )
        .await;
    }

    async fn syncfs(this: &AsyncRc<Self>, req: Request, body: request::SyncFs) {
        let id = req.id;
        let req = req::SyncFsReq::new(Req::new(this, req), body);
        this.send(id, this.fs.syncfs(req).await).await;
    }

    async fn tmpfile(this: &AsyncRc<Self>, req: Request, body: request::TmpFile) {
        let id = req.id;
        let req = req::TmpFileReq::new(Req::new(this, req), body);
        this.send(id, this.fs.tmpfile(req).await).await;
    }

    async fn statx(this: &AsyncRc<Self>, req: Request, body: request::StatX) {
        let id = req.id;
        let req = req::StatXReq::new(Req::new(this, req), body);
        this.send(id, this.fs.statx(req).await).await;
    }

    async fn copy_file_range64(this: &AsyncRc<Self>, req: Request, body: request::CopyFileRange64) {
        let id = req.id;
        let req = req::CopyFileRangeReq::new(Req::new(this, req), body);
        this.send(
            id,
            this.fs.copy_file_range(req).await.map(CopyFileRange::new),
        )
        .await;
    }

    #[cfg(target_os = "macos")]
    async fn setvolname(this: &AsyncRc<Self>, req: Request, body: request::SetVolName) {
        todo!()
    }

    #[cfg(target_os = "macos")]
    async fn getxtimes(this: &AsyncRc<Self>, req: Request, body: request::GetXtimes) {
        todo!()
    }

    async fn send<T>(&self, id: u64, resp: T)
    where
        T: EncodeResp,
        Error: From<T::Error>,
    {
        let cfg = crate::proto::Cfg {
            minor_ver: self.inner.minor_ver,
            flags: self.inner.flags,
        };

        let buf = match resp.encode(id, cfg) {
            Ok(buf) => buf.left_buf(),
            Err(err) => {
                let Ok(buf) = Error::from(err).encode(id, cfg);
                buf.right_buf()
            }
        };

        if let BufResult(Err(err), _) = self.inner.dev.write_buf(buf).await {
            log::error!("worker {} failed to send response on FUSE device: {}", self.inner.id, err);
        }
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
