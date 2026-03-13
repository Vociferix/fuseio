use super::{Mount, Unmount};
use crate::MountOpt;

use std::io::Result;
use std::os::fd::BorrowedFd;
use std::path::Path;

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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DirectMount;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DirectUnmount {
    _priv: (),
}

impl Mount for DirectMount {
    type Unmount = DirectUnmount;

    async fn mount(
        &self,
        dev: BorrowedFd<'_>,
        mountpoint: &Path,
        options: &[MountOpt],
    ) -> Result<Self::Unmount> {
        sys::mount(dev, mountpoint, options).await?;
        Ok(DirectUnmount { _priv: () })
    }
}

impl Unmount for DirectUnmount {
    async fn unmount(
        self,
        dev: BorrowedFd<'_>,
        mountpoint: &Path,
        options: &[MountOpt],
    ) -> Result<()> {
        sys::unmount(dev, mountpoint, options).await
    }
}
