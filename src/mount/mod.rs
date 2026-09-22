use crate::MountOpt;

use std::io::Result;
use std::os::fd::{BorrowedFd, OwnedFd};
use std::path::Path;

#[cfg(not(target_os = "macos"))]
mod direct;

#[cfg(any(
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly",
))]
mod mount_fusefs;

#[cfg(not(any(
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly",
)))]
mod fusermount;

mod default;

#[cfg(not(target_os = "macos"))]
pub use direct::{DirectMount, DirectUnmount};

#[cfg(any(
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly",
))]
pub use mount_fusefs::{MountFuseFs, MountFuseFsUnmount};

#[cfg(not(any(
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly",
)))]
pub use fusermount::{Fusermount, FusermountUnmount};

pub use default::{DefaultMount, DefaultUnmount};

pub trait Mount: 'static {
    type Unmount: Unmount;

    async fn mount(
        &self,
        mountpoint: &Path,
        options: &[MountOpt],
    ) -> Result<(OwnedFd, impl Future<Output = Result<Self::Unmount>>)>;
}

pub trait Unmount: 'static {
    async fn unmount(
        self,
        dev: BorrowedFd<'_>,
        mountpoint: &Path,
        options: &[MountOpt],
    ) -> Result<()>;
}
