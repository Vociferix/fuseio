use crate::buf_pool::{BufGuard, BufPool};

use compio::BufResult;
use compio::driver::{OwnedFd, op::OpenFile};
use compio::fs::AsyncFd;
use compio::io::{AsyncRead, AsyncWriteExt};
use compio::runtime::submit;

use std::io::Result;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, RawFd};
use std::path::Path;

const BUF_SIZE: usize = crate::MAX_WRITE_SIZE + 4096;

#[derive(Debug, Clone)]
pub struct Sender {
    dev: AsyncFd<OwnedFd>,
}

#[derive(Debug, Clone)]
pub struct Receiver {
    dev: AsyncFd<OwnedFd>,
    bufs: BufPool,
}

pub async fn channel<P>(path: P) -> Result<(Sender, Receiver)>
where
    P: AsRef<Path>,
{
    let op = OpenFile::new(path_string(path)?, nix::libc::O_RDWR, 0o666);

    let fd = submit(op).await.0? as RawFd;

    // SAFETY: The above call to the `open` syscall (via compio) has returned
    //         a valid file descriptor, so it is safe to take ownership, and
    //         necessary to ensure the FD is closed on drop.
    let fd = unsafe { OwnedFd::from_raw_fd(fd) };

    let fd = AsyncFd::new(fd)?;

    let tx = Sender { dev: fd.clone() };
    let rx = Receiver {
        dev: fd,
        bufs: BufPool::new(),
    };

    Ok((tx, rx))
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

impl Sender {
    pub async fn send_buf(&mut self, buf: &mut Vec<u8>) -> std::io::Result<()> {
        let BufResult(res, tmp_buf) = self.dev.write_all(core::mem::take(buf)).await;
        *buf = tmp_buf;
        res
    }

    pub async fn send<T>(&mut self, data: T) -> std::io::Result<()>
    where
        T: bytemuck::Pod + 'static,
    {
        struct Buf<T>(T);

        impl<T> compio::buf::IoBuf for Buf<T>
        where
            T: bytemuck::Pod + 'static,
        {
            fn as_init(&self) -> &[u8] {
                bytemuck::bytes_of(&self.0)
            }
        }

        self.dev.write_all(Buf(data)).await.0
    }
}

impl AsFd for Sender {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.dev.as_fd()
    }
}

impl AsRawFd for Sender {
    fn as_raw_fd(&self) -> RawFd {
        self.dev.as_raw_fd()
    }
}

impl Receiver {
    pub async fn recv(&mut self) -> std::io::Result<BufGuard<u8>> {
        let buf = self.bufs.checkout_with_capacity::<u8>(BUF_SIZE);

        let compio::BufResult(res, mut buf) = self.dev.read(buf).await;
        let len = res?;
        unsafe {
            buf.set_len(len);
        }

        Ok(buf)
    }
}

impl AsFd for Receiver {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.dev.as_fd()
    }
}

impl AsRawFd for Receiver {
    fn as_raw_fd(&self) -> RawFd {
        self.dev.as_raw_fd()
    }
}
