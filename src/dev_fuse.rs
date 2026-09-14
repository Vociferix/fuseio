use crate::ioctl::clone_fd;
use crate::types::{Mode, OFlag};

use compio::buf::{BufResult, IoBuf, IoBufMut, IoVectoredBuf};
use compio::io::{AsyncRead, AsyncWrite};
use compio::runtime::fd::AsyncFd;

use std::io::Result;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, IntoRawFd, OwnedFd, RawFd};
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Debug)]
pub struct DevFuse {
    fd: OwnedFd,
    path: Arc<Path>,
}

#[derive(Debug, Clone)]
pub struct FuseChannel {
    dev: AsyncFd<DevFuse>,
}

impl DevFuse {
    pub async fn open<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let path: Arc<Path> = path.as_ref().into();

        // This could probably be implemented asynchronously without
        // spawn_blocking, but this should be very infrequently called
        // (once per filesystem mount), so the overhead is acceptable.
        // It also makes sure we don't attach the FD to the current
        // runtime yet.
        compio::runtime::spawn_blocking(move || -> Result<Self> {
            let fd = nix::fcntl::open(
                path.as_ref(),
                OFlag::O_RDWR | OFlag::O_CLOEXEC,
                Mode::empty(),
            )?;
            Ok(Self { fd, path })
        })
        .await
        .unwrap()
    }

    pub async fn try_clone(&self) -> Result<Self> {
        let fd = self.fd.as_raw_fd();
        let path = self.path.clone();
        compio::runtime::spawn_blocking(move || Self::try_clone_impl(fd, path))
            .await
            .unwrap()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn bind(self) -> Result<FuseChannel> {
        Ok(FuseChannel {
            dev: AsyncFd::new(self)?,
        })
    }

    #[cfg(target_os = "linux")]
    fn try_clone_impl(oldfd: RawFd, path: Arc<Path>) -> Result<Self> {
        let fd = nix::fcntl::open(&*path, OFlag::O_RDWR | OFlag::O_CLOEXEC, Mode::empty())?;
        let mut fd = fd.into_raw_fd();

        let res = unsafe { clone_fd(oldfd, &mut fd) };
        let fd = unsafe { OwnedFd::from_raw_fd(fd) };
        res?;

        Ok(Self {
            fd,
            path: path.clone(),
        })
    }

    #[cfg(not(target_os = "linux"))]
    fn try_clone_impl(oldfd: RawFd, path: Arc<Path>) -> Result<Self> {
        let oldfd = std::mem::ManuallyDrop::new(unsafe { OwnedFd::from_raw_fd(oldfd) });
        let fd = nix::unistd::dup(&*oldfd)?;

        nix::fcntl::fcntl(
            &fd,
            nix::fcntl::FcntlArg::F_SETFD(nix::fcntl::FdFlag::FD_CLOEXEC),
        )?;

        Ok(Self { fd, path })
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
