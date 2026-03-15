use crate::buf_pool::Buf;

use compio::BufResult;
use compio::driver::{OwnedFd, SharedFd, op::OpenFile};
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

#[derive(Debug)]
pub struct Receiver {
    dev: AsyncFd<OwnedFd>,
    buf: Buf<u8>,
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
        buf: Buf::with_capacity(BUF_SIZE),
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
    pub async fn recv(&mut self) -> std::io::Result<&[u8]> {
        let mut buf = std::mem::take(&mut self.buf);
        buf.clear();
        buf.reserve(BUF_SIZE);

        let compio::BufResult(res, buf) = self.dev.read(buf).await;
        self.buf = buf;
        let len = res?;
        unsafe {
            self.buf.set_len(len);
        }

        Ok(&self.buf)
    }
}

impl Clone for Receiver {
    fn clone(&self) -> Self {
        Self {
            dev: self.dev.clone(),
            buf: Buf::with_capacity(BUF_SIZE),
        }
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
