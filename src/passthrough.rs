use crate::dev_fuse::FuseChannel;
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
    pub(crate) fn open(fd: T, dev: &FuseChannel) -> crate::Result<Self> {
        let map = BackingMap {
            fd: fd.as_fd().as_raw_fd(),
            flags: 0,
            _unused: 0,
        };

        let res = unsafe { passthrough_open(dev.as_raw_fd(), &map) };

        let backing_id = match res {
            Ok(backing_id) => backing_id.cast_unsigned(),
            Err(err) => {
                return Err(
                    crate::Error::from_raw_os_error(err as i32).unwrap_or(crate::Error::EBADF)
                );
            }
        };

        Ok(Self {
            backing_id,
            fd,
            dev: dev.clone(),
        })
    }

    pub fn close(this: Self) -> crate::Result<T> {
        let this = ManuallyDrop::new(this);
        let backing_id = this.backing_id;
        let fd = unsafe { std::ptr::read(&this.fd) };
        let dev = unsafe { std::ptr::read(&this.dev) };

        let res = unsafe { passthrough_close(dev.as_raw_fd(), &backing_id) };

        if let Err(err) = res {
            return Err(crate::Error::from_raw_os_error(err as i32).unwrap_or(crate::Error::EBADF));
        };

        Ok(fd)
    }

    pub fn backing_id(this: &Self) -> BackingId {
        BackingId(this.backing_id)
    }
}

impl<T: AsFd> Drop for PassthroughFd<T> {
    fn drop(&mut self) {
        unsafe {
            let _ = passthrough_close(self.dev.as_raw_fd(), &self.backing_id);
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
