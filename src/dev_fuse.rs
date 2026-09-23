use crate::buf::{IntoIoBuf, IoBuffer};
use crate::ioctl::clone_fd;
use crate::types::{Mode, OFlag};

use compio::buf::{BufResult, IoBuf, IoBufMut, IoVectoredBuf};
use compio::io::{AsyncRead, AsyncWrite};
use compio::runtime::fd::AsyncFd;

use std::io::Result;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, IntoRawFd, OwnedFd, RawFd};

#[derive(Debug)]
pub struct DevFuse {
    fd: OwnedFd,
}

#[derive(Debug, Clone)]
pub struct FuseChannel {
    dev: AsyncFd<DevFuse>,
}

impl DevFuse {
    // TODO: the fd is never made O_NONBLOCK. compio's poll-based drivers
    // (FreeBSD, macOS, Linux without io_uring) wait for readability and then
    // read(), so with `dup`'d fds sharing one queue, every worker wakes but only
    // one gets the request; the rest block their thread in read().
    // TODO(e2e): kqueue support for /dev/macfuseN (and the FSKit socket) is
    // unverified.
    pub fn new(fd: OwnedFd) -> Self {
        Self { fd }
    }

    pub async fn try_clone(&self) -> Result<Self> {
        let fd = self.fd.as_raw_fd();
        compio::runtime::spawn_blocking(move || Self::try_clone_impl(fd))
            .await
            .unwrap()
    }

    pub fn bind(self) -> Result<FuseChannel> {
        Ok(FuseChannel {
            dev: AsyncFd::new(self)?,
        })
    }

    #[cfg(target_os = "linux")]
    fn try_clone_impl(oldfd: RawFd) -> Result<Self> {
        use std::io::Write;

        const PATHBUF_MAX_LEN: usize = "/proc/self/fd/2147483647".len();
        let mut pathbuf: arrayvec::ArrayVec<u8, PATHBUF_MAX_LEN> = arrayvec::ArrayVec::new();
        write!(pathbuf, "/proc/self/fd/{oldfd}")?;

        let Ok(fd) = nix::fcntl::open(
            pathbuf.as_slice(),
            OFlag::O_RDWR | OFlag::O_CLOEXEC,
            Mode::empty(),
        ) else {
            return Self::try_clone_dup(oldfd);
        };
        let mut fd = fd.into_raw_fd();

        let res = unsafe { clone_fd(oldfd, &mut fd) };
        let fd = unsafe { OwnedFd::from_raw_fd(fd) };

        if res.is_err() {
            drop(fd);
            return Self::try_clone_dup(oldfd);
        }

        Ok(Self { fd })
    }

    #[cfg(not(target_os = "linux"))]
    fn try_clone_impl(oldfd: RawFd) -> Result<Self> {
        Self::try_clone_dup(oldfd)
    }

    fn try_clone_dup(oldfd: RawFd) -> Result<Self> {
        let oldfd = std::mem::ManuallyDrop::new(unsafe { OwnedFd::from_raw_fd(oldfd) });
        let fd = nix::unistd::dup(&*oldfd)?;

        nix::fcntl::fcntl(
            &fd,
            nix::fcntl::FcntlArg::F_SETFD(nix::fcntl::FdFlag::FD_CLOEXEC),
        )?;

        Ok(Self { fd })
    }
}

impl AsFd for DevFuse {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl AsRawFd for DevFuse {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

impl FuseChannel {
    pub async fn read<B>(&self, buf: B) -> BufResult<usize, B>
    where
        B: IoBufMut,
    {
        (&self.dev).read(buf).await
    }

    pub async fn write<B>(&self, buf: B) -> BufResult<usize, B>
    where
        B: IoBuf,
    {
        (&self.dev).write(buf).await
    }

    pub async fn write_vectored<B>(&self, buf: B) -> BufResult<usize, B>
    where
        B: IoVectoredBuf,
    {
        (&self.dev).write_vectored(buf).await
    }

    pub async fn write_buf<B>(&self, buf: B) -> BufResult<usize, IoBuffer<B::Buffer, B::VecBuffer>>
    where
        B: IntoIoBuf,
    {
        match buf.into_io_buf() {
            IoBuffer::Buf(buf) => self.write(buf).await.map_buffer(IoBuffer::Buf),
            IoBuffer::VecBuf(buf) => self.write_vectored(buf).await.map_buffer(IoBuffer::VecBuf),
        }
    }
}

impl std::ops::Deref for FuseChannel {
    type Target = DevFuse;

    fn deref(&self) -> &Self::Target {
        &*self.dev
    }
}

impl AsFd for FuseChannel {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.dev.as_fd()
    }
}

impl AsRawFd for FuseChannel {
    fn as_raw_fd(&self) -> RawFd {
        self.dev.as_raw_fd()
    }
}
