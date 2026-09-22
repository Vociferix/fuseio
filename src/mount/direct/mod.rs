use super::{Mount, Unmount};
use crate::MountOpt;

use std::io::Result;
use std::os::fd::{BorrowedFd, OwnedFd};
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DirectMount {
    dev_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DirectUnmount {
    _priv: (),
}

impl Mount for DirectMount {
    type Unmount = DirectUnmount;

    async fn mount(
        &self,
        mountpoint: &Path,
        options: &[MountOpt],
    ) -> Result<(OwnedFd, impl Future<Output = Result<Self::Unmount>>)> {
        let dev_path = self.dev_path.as_deref().unwrap_or("/dev/fuse".as_ref());
        let fd = sys::mount(dev_path, mountpoint, options).await?;
        Ok((fd, std::future::ready(Ok(DirectUnmount { _priv: () }))))
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
