use crate::conn::Connection;
use crate::context::Context;

use std::mem::ManuallyDrop;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, RawFd};

#[derive(Debug)]
pub struct PassthroughFd<T: AsFd, C: Connection> {
    backing_id: u32,
    fd: T,
    ctx: Context<C>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct RawBackingId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BackingId(pub(crate) u32);

impl<T: AsFd, C: Connection> PassthroughFd<T, C> {
    pub(crate) async fn open(fd: T, ctx: &Context<C>) -> crate::Result<Self> {
        let backing_id = ctx.server.conn.open_passthrough(&fd).await?.0;

        Ok(Self {
            backing_id,
            fd,
            ctx: Context::clone(ctx),
        })
    }

    /// Stops the kernel reading and writing the backing file, returning it.
    pub fn close(this: Self) -> crate::Result<T> {
        let this = ManuallyDrop::new(this);
        let backing_id = this.backing_id;
        let fd = unsafe { std::ptr::read(&this.fd) };
        let ctx = unsafe { std::ptr::read(&this.ctx) };

        ctx.server
            .conn
            .close_passthrough(RawBackingId(backing_id))?;

        Ok(fd)
    }

    pub fn backing_id(this: &Self) -> BackingId {
        BackingId(this.backing_id)
    }
}

impl<T: AsFd, C: Connection> Drop for PassthroughFd<T, C> {
    fn drop(&mut self) {
        // A backing file the kernel won't let go of stays open until the
        // connection ends, so say so rather than dropping the error.
        if let Err(err) = self
            .ctx
            .server
            .conn
            .close_passthrough(RawBackingId(self.backing_id))
        {
            log::error!("failed to close backing file {}: {err}", self.backing_id);
        }
    }
}

impl<T: AsFd, C: Connection> std::ops::Deref for PassthroughFd<T, C> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.fd
    }
}

impl<T: AsFd, C: Connection> std::ops::DerefMut for PassthroughFd<T, C> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.fd
    }
}

impl<T: AsFd, C: Connection> AsFd for PassthroughFd<T, C> {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl<T: AsFd + AsRawFd, C: Connection> AsRawFd for PassthroughFd<T, C> {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

impl BackingId {
    pub const fn as_raw(self) -> u32 {
        self.0
    }
}
