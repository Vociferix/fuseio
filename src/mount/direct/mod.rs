//! Mounting by calling `mount(2)` ourselves, with no setuid helper.
//!
//! This is what a process that may already mount in its namespace wants: it
//! opens the device and asks the kernel directly, so there is no fork, no
//! `exec`, and no descriptor handed over a socket. A process without that
//! privilege gets `EPERM` from the kernel and should use
//! [`Fusermount`](super::Fusermount) instead.
//!
//! The permission and policy checks that make up most of libfuse's
//! `fusermount` are deliberately absent. They exist because `fusermount` is
//! setuid root and must decide, in userspace, what an unprivileged caller may
//! ask for; here the kernel sees the caller's own credentials and answers that
//! question itself. See [`sys`] for the checks that were kept, and why.

use super::{Mount, Unmount};
use crate::MountOpt;
use crate::conn::{Conn, DevFuseConn, DevFuseSharedConn};

use std::io::Result;
use std::os::fd::AsFd;
use std::path::{Path, PathBuf};

#[cfg_attr(target_os = "linux", path = "linux.rs")]
#[cfg_attr(
    any(
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly",
    ),
    path = "bsd.rs"
)]
mod sys;

/// Where the FUSE device lives when none was named. Every platform this module
/// is compiled for uses the same path; macFUSE's per-mount devices are reached
/// through its own mounter.
const DEFAULT_DEVICE: &str = "/dev/fuse";

/// A [`Mount`] implementation that calls `mount(2)` without a helper.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DirectMount {
    dev_path: Option<PathBuf>,
}

/// The unmount half of [`DirectMount`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DirectUnmount {
    _priv: (),
}

impl DirectMount {
    /// Constructs a mounter that uses the platform's usual FUSE device.
    pub const fn new() -> Self {
        Self { dev_path: None }
    }

    /// Constructs a mounter that opens the device at `path` instead of the
    /// platform's usual one.
    pub fn with_device<P>(path: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            dev_path: Some(path.as_ref().to_path_buf()),
        }
    }

    /// The device to open.
    fn device(&self) -> &Path {
        self.dev_path
            .as_deref()
            .unwrap_or(Path::new(DEFAULT_DEVICE))
    }
}

impl Mount for DirectMount {
    type SharedConn = DevFuseSharedConn;
    type Conn = DevFuseConn;
    type Unmount = DirectUnmount;

    async fn mount(
        &self,
        mountpoint: &Path,
        options: &[MountOpt],
        num_workers: usize,
    ) -> Result<(
        Self::SharedConn,
        impl Future<Output = Result<Self::Unmount>>,
    )> {
        // One device is opened here, and further workers clone it themselves,
        // so the count says nothing about this step.
        let _ = num_workers;

        let device = self.device();
        let conn = DevFuseSharedConn::open_at(device).await?;

        // Dropping `conn` on the way out closes the device, which is what
        // tells the kernel the mount never happened.
        sys::attach(conn.as_fd(), device, mountpoint, options)?;

        // `mount(2)` returns once the superblock exists, so there is nothing
        // left to wait for.
        Ok((conn, std::future::ready(Ok(DirectUnmount { _priv: () }))))
    }
}

impl Unmount for DirectUnmount {
    type SharedConn = DevFuseSharedConn;
    type Conn = DevFuseConn;

    async fn unmount(
        self,
        conn: Conn<Self::SharedConn>,
        mountpoint: &Path,
        options: &[MountOpt],
    ) -> Result<()> {
        // The options described the mount that is going away; unmounting takes
        // only the path.
        let _ = options;

        // Closing the device first is not optional: a synchronous unmount of a
        // filesystem whose server still holds the device recurses into that
        // filesystem and deadlocks.
        drop(conn);

        sys::detach(mountpoint)
    }
}
