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

    /// Hands the device to the runtime, which reads and writes it without
    /// blocking.
    pub fn bind(self) -> Result<FuseChannel> {
        // compio's poll-based drivers wait for readability and then read, so a
        // worker whose readiness was consumed by another sharing the same open
        // file would otherwise block its thread. compio doesn't set this itself.
        set_nonblocking(self.as_fd())?;

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

fn set_nonblocking(fd: BorrowedFd<'_>) -> Result<()> {
    let flags = nix::fcntl::fcntl(fd, nix::fcntl::FcntlArg::F_GETFL)?;
    let flags = OFlag::from_bits_retain(flags) | OFlag::O_NONBLOCK;

    nix::fcntl::fcntl(fd, nix::fcntl::FcntlArg::F_SETFL(flags))?;

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    use std::os::fd::OwnedFd;

    fn is_nonblocking(fd: BorrowedFd<'_>) -> bool {
        let flags = nix::fcntl::fcntl(fd, nix::fcntl::FcntlArg::F_GETFL).unwrap();

        OFlag::from_bits_retain(flags).contains(OFlag::O_NONBLOCK)
    }

    fn pipe() -> (OwnedFd, OwnedFd) {
        nix::unistd::pipe().unwrap()
    }

    #[test]
    fn a_device_starts_blocking() {
        let (read, _write) = pipe();

        assert!(!is_nonblocking(read.as_fd()));
    }

    #[test]
    fn binding_makes_the_device_nonblocking() {
        let (read, _write) = pipe();

        set_nonblocking(read.as_fd()).unwrap();

        assert!(is_nonblocking(read.as_fd()));
    }

    #[test]
    fn the_other_flags_are_kept() {
        // Something with flags worth losing: the access mode and O_APPEND.
        let file = std::fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open("/dev/null")
            .unwrap();

        let before = nix::fcntl::fcntl(file.as_fd(), nix::fcntl::FcntlArg::F_GETFL).unwrap();
        let before = OFlag::from_bits_retain(before);

        assert!(before.contains(OFlag::O_APPEND));
        assert!(!before.contains(OFlag::O_NONBLOCK));

        set_nonblocking(file.as_fd()).unwrap();

        let after = nix::fcntl::fcntl(file.as_fd(), nix::fcntl::FcntlArg::F_GETFL).unwrap();
        let after = OFlag::from_bits_retain(after);

        assert!(after.contains(OFlag::O_NONBLOCK));
        assert!(after.contains(OFlag::O_APPEND));
        assert_eq!(
            after & OFlag::O_ACCMODE,
            before & OFlag::O_ACCMODE,
            "the access mode changed"
        );
    }

    // A `dup` shares the open file, which is how the workers that can't use the
    // clone ioctl get the flag.
    #[test]
    fn a_dup_shares_the_flag() {
        let (read, _write) = pipe();
        let clone = nix::unistd::dup(&read).unwrap();

        set_nonblocking(read.as_fd()).unwrap();

        assert!(is_nonblocking(clone.as_fd()));
    }

    #[test]
    fn setting_it_twice_is_harmless() {
        let (read, _write) = pipe();

        set_nonblocking(read.as_fd()).unwrap();
        set_nonblocking(read.as_fd()).unwrap();

        assert!(is_nonblocking(read.as_fd()));
    }
}
