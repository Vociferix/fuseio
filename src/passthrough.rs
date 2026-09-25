use crate::dev_fuse::FuseChannel;
#[cfg(target_os = "linux")]
use crate::ioctl::{BackingMap, passthrough_close, passthrough_open};

use std::mem::ManuallyDrop;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, RawFd};

#[derive(Debug)]
pub struct PassthroughFd<T: AsFd> {
    backing_id: u32,
    fd: T,
    dev: FuseChannel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BackingId(pub(crate) u32);

impl<T: AsFd> PassthroughFd<T> {
    #[cfg(target_os = "linux")]
    pub(crate) fn open(fd: T, dev: &FuseChannel) -> crate::Result<Self> {
        let map = BackingMap {
            fd: fd.as_fd().as_raw_fd(),
            flags: 0,
            padding: 0,
        };

        let res = unsafe { passthrough_open(dev.as_raw_fd(), &map) };

        let backing_id = res?.cast_unsigned();

        Ok(Self {
            backing_id,
            fd,
            dev: dev.clone(),
        })
    }

    /// Only Linux has passthrough: FreeBSD's device answers no ioctls at all,
    /// and the macOS kernel extension has no such feature.
    #[cfg(not(target_os = "linux"))]
    pub(crate) fn open(fd: T, dev: &FuseChannel) -> crate::Result<Self> {
        let _ = (fd, dev);

        Err(crate::Error::ENOTSUP)
    }

    /// Stops the kernel reading and writing the backing file, returning it.
    pub fn close(this: Self) -> crate::Result<T> {
        let this = ManuallyDrop::new(this);
        let backing_id = this.backing_id;
        let fd = unsafe { std::ptr::read(&this.fd) };
        let dev = unsafe { std::ptr::read(&this.dev) };

        Self::close_backing(&dev, backing_id)?;

        Ok(fd)
    }

    #[cfg(target_os = "linux")]
    fn close_backing(dev: &FuseChannel, backing_id: u32) -> crate::Result<()> {
        unsafe { passthrough_close(dev.as_raw_fd(), &backing_id) }?;

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    fn close_backing(dev: &FuseChannel, backing_id: u32) -> crate::Result<()> {
        let _ = (dev, backing_id);

        Err(crate::Error::ENOTSUP)
    }

    pub fn backing_id(this: &Self) -> BackingId {
        BackingId(this.backing_id)
    }
}

impl<T: AsFd> Drop for PassthroughFd<T> {
    fn drop(&mut self) {
        // A backing file the kernel won't let go of stays open until the
        // connection ends, so say so rather than dropping the error.
        if let Err(err) = Self::close_backing(&self.dev, self.backing_id) {
            log::error!("failed to close backing file {}: {err}", self.backing_id);
        }
    }
}

impl<T: AsFd> std::ops::Deref for PassthroughFd<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.fd
    }
}

impl<T: AsFd> std::ops::DerefMut for PassthroughFd<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.fd
    }
}

impl<T: AsFd> AsFd for PassthroughFd<T> {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl<T: AsFd + AsRawFd> AsRawFd for PassthroughFd<T> {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

impl BackingId {
    pub const fn as_raw(self) -> u32 {
        self.0
    }
}
