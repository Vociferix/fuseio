use super::{
    BufResult, ConnCaps, Connection, ConnectionMeta, IoBuf, IoBufMut, IoVectoredBuf, NotifyCaps,
    Result, SharedConnection,
};
use crate::types::OFlag;

#[cfg(target_os = "linux")]
use super::RawBackingId;

use compio::driver::op::{CurrentDir, Mode, OFlags, OpenFile};
use compio::io::{AsyncRead, AsyncWrite};
use compio::runtime::fd::AsyncFd;
use compio::runtime::submit;

use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd, RawFd};

#[cfg(target_os = "linux")]
mod ioctl;

#[derive(Debug)]
pub struct DevFuseConn {
    fd: AsyncFd<OwnedFd>,
}

#[derive(Debug)]
pub struct DevFuseSharedConn {
    fd: OwnedFd,
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
        (&self.fd).read(buf).await.map_res(|len| (len, ()))
    }

    async fn send_response<B>(&self, token: Self::ReqToken, buf: B) -> BufResult<(), B>
    where
        B: IoBuf,
    {
        #[allow(clippy::let_unit_value)]
        let _ = token;

        self.send_notification(buf).await
    }

    async fn send_response_vectored<B>(&self, token: Self::ReqToken, buf: B) -> BufResult<(), B>
    where
        B: IoVectoredBuf,
    {
        #[allow(clippy::let_unit_value)]
        let _ = token;

        self.send_notification_vectored(buf).await
    }

    async fn send_notification<B>(&self, buf: B) -> BufResult<(), B>
    where
        B: IoBuf,
    {
        (&self.fd).write(buf).await.map_res(|_| ())
    }

    async fn send_notification_vectored<B>(&self, buf: B) -> BufResult<(), B>
    where
        B: IoVectoredBuf,
    {
        (&self.fd).write_vectored(buf).await.map_res(|_| ())
    }

    #[cfg(target_os = "linux")]
    async fn open_passthrough<T>(&self, fd: T) -> Result<RawBackingId>
    where
        T: AsFd,
    {
        let map = ioctl::BackingMap {
            fd: fd.as_fd().as_raw_fd(),
            flags: 0,
            padding: 0,
        };

        let backing_id = unsafe { ioctl::passthrough_open(self.as_raw_fd(), &map)? };

        Ok(RawBackingId(backing_id.cast_unsigned()))
    }

    #[cfg(target_os = "linux")]
    fn close_passthrough(&self, backing_id: RawBackingId) -> Result<()> {
        unsafe {
            ioctl::passthrough_close(self.as_raw_fd(), &backing_id.0)?;
        }
        Ok(())
    }
}

impl AsFd for DevFuseConn {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl AsRawFd for DevFuseConn {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
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

impl DevFuseSharedConn {
    pub async fn open() -> Result<Self> {
        Self::open_at("/dev/fuse").await
    }

    pub async fn open_at<P>(path: P) -> Result<Self>
    where
        P: AsRef<std::path::Path>,
    {
        let op = OpenFile::new(
            CurrentDir,
            path_string(path)?,
            OFlags::RDWR | OFlags::CLOEXEC,
            Mode::from_bits_retain(0o666),
        );

        let fd = submit(op).await.0? as RawFd;

        // SAFETY: The above call to the `openat` syscall (via compio) has returned
        //         a valid file descriptor, so it is safe to take ownership, and
        //         necessary to ensure the FD is closed on drop.
        let fd = unsafe { OwnedFd::from_raw_fd(fd) };

        Self::from_fd(fd)
    }

    /// Takes ownership of an already-open FUSE device.
    ///
    /// The device is put into non-blocking mode, which is an invariant of this
    /// type rather than something its callers arrange: a blocking device can
    /// stall the thread running the reactor. Both drivers wait for readability
    /// and then read, and the read sleeps rather than failing if the request it
    /// was woken for has meanwhile been taken by another reader of the same
    /// queue. `fuse_dev_do_read` honours `O_NONBLOCK` but ignores `IOCB_NOWAIT`,
    /// so the open file description's flag is the only thing that turns that
    /// sleep into an `EAGAIN` the reactor can wait on again.
    pub fn from_fd(fd: OwnedFd) -> Result<Self> {
        set_nonblocking(&fd, true)?;

        Ok(Self { fd })
    }

    /// Gives up ownership of the device, leaving it as it was found.
    ///
    /// Non-blocking mode belongs to this type, so it is undone here; a caller
    /// that wants it can set it again.
    pub fn into_fd(self) -> Result<OwnedFd> {
        set_nonblocking(&self.fd, false)?;

        Ok(self.fd)
    }

    /// Reopens the device and joins it to this connection, giving the clone its
    /// own request queue.
    ///
    /// There is deliberately no fallback to [`dup`](Self::dup): a duplicate
    /// shares the open file description, and so the queue, which is the opposite
    /// of what [`ConnCaps::INDEPENDENT_CLONES`] promises. It also hides a failed
    /// ioctl, which is how a reversed argument here went unnoticed once already.
    /// The cost is that `FUSE_DEV_IOC_CLONE` (Linux 4.2) is required whenever
    /// more than one worker is asked for.
    #[cfg(target_os = "linux")]
    async fn try_clone_impl(&self) -> Result<Self> {
        use std::ffi::OsStr;
        use std::io::Write;
        use std::os::unix::ffi::OsStrExt;

        const PATHBUF_MAX_LEN: usize = "/proc/self/fd/2147483647".len();

        let mut pathbuf: arrayvec::ArrayVec<u8, PATHBUF_MAX_LEN> = arrayvec::ArrayVec::new();
        write!(pathbuf, "/proc/self/fd/{}", self.as_raw_fd())?;

        let out = Self::open_at(OsStr::from_bytes(pathbuf.as_slice())).await?;

        // The ioctl goes to the *new* fd, naming this one as the master.
        let mut master = self.as_raw_fd().cast_unsigned();
        unsafe {
            ioctl::clone_fd(out.as_raw_fd(), &mut master)?;
        }

        Ok(out)
    }

    /// Duplicates the device, which shares one request queue between the two.
    ///
    /// No other platform has `FUSE_DEV_IOC_CLONE`, so extra workers here take
    /// turns on a single queue rather than getting their own; hence no
    /// [`ConnCaps::INDEPENDENT_CLONES`] off Linux.
    #[cfg(not(target_os = "linux"))]
    async fn try_clone_impl(&self) -> Result<Self> {
        Self::from_fd(self.dup()?)
    }

    fn dup(&self) -> Result<OwnedFd> {
        let fd = nix::unistd::dup(self)?;

        nix::fcntl::fcntl(
            &fd,
            nix::fcntl::FcntlArg::F_SETFD(nix::fcntl::FdFlag::FD_CLOEXEC),
        )?;

        Ok(fd)
    }

    /// A duplicate of the device, ready for one asynchronous operation.
    ///
    /// The duplicate rather than the device itself, because an operation
    /// outlives the call that submitted it when that call is cancelled: the
    /// driver holds the descriptor until the kernel reports the cancellation.
    /// Handing it an owned duplicate makes that harmless, where lending it this
    /// connection's descriptor would need a completion this method has no way to
    /// await.
    ///
    /// The duplicate shares the open file description, so it is non-blocking
    /// for free and names the same request queue.
    ///
    /// Only the handshake goes through here, so the extra `dup` is a few
    /// syscalls over the life of a mount. A bound [`DevFuseConn`] keeps one
    /// descriptor for all of its work.
    fn attach_dup(&self) -> Result<AsyncFd<OwnedFd>> {
        AsyncFd::new(self.dup()?)
    }
}

impl SharedConnection for DevFuseSharedConn {
    type Bound = DevFuseConn;

    async fn bind(self) -> Result<Self::Bound> {
        Ok(DevFuseConn {
            fd: AsyncFd::new(self.fd)?,
        })
    }

    async fn try_clone(&self) -> Result<Self> {
        self.try_clone_impl().await
    }

    async fn recv_request<B>(&self, buf: B) -> BufResult<(usize, Self::ReqToken), B>
    where
        B: IoBufMut + Send,
    {
        let fd = match self.attach_dup() {
            Ok(fd) => fd,
            Err(err) => return BufResult(Err(err), buf),
        };

        (&fd).read(buf).await.map_res(|len| (len, ()))
    }

    async fn send_response<B>(&self, token: Self::ReqToken, buf: B) -> BufResult<(), B>
    where
        B: IoBuf + Send,
    {
        #[allow(clippy::let_unit_value)]
        let _ = token;

        let fd = match self.attach_dup() {
            Ok(fd) => fd,
            Err(err) => return BufResult(Err(err), buf),
        };

        (&fd).write(buf).await.map_res(|_| ())
    }
}

impl AsFd for DevFuseSharedConn {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl AsRawFd for DevFuseSharedConn {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

/// Turns non-blocking mode on or off for an open file description.
fn set_nonblocking<F: AsFd>(fd: F, on: bool) -> Result<()> {
    let flags = nix::fcntl::fcntl(&fd, nix::fcntl::FcntlArg::F_GETFL)?;
    let mut flags = OFlag::from_bits_retain(flags);

    flags.set(OFlag::O_NONBLOCK, on);

    nix::fcntl::fcntl(&fd, nix::fcntl::FcntlArg::F_SETFL(flags))?;

    Ok(())
}

fn path_string(path: impl AsRef<std::path::Path>) -> Result<std::ffi::CString> {
    use std::os::unix::ffi::OsStrExt;

    std::ffi::CString::new(path.as_ref().as_os_str().as_bytes().to_vec()).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "FUSE device path contained an unexpected NUL byte",
        )
    })
}
