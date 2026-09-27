use crate::MountOpt;
use crate::conn::{Connection, SharedConnection};

use std::io::Result;
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
    type SharedConn: SharedConnection<Bound = Self::Conn>;
    type Conn: Connection;
    type Unmount: Unmount<SharedConn = Self::SharedConn, Conn = Self::Conn>;

    async fn mount(
        &self,
        mountpoint: &Path,
        options: &[MountOpt],
        num_workers: usize,
    ) -> Result<(
        Self::SharedConn,
        impl Future<Output = Result<Self::Unmount>>,
    )>;
}

pub trait Unmount: 'static {
    type SharedConn: SharedConnection<Bound = Self::Conn>;
    type Conn: Connection;

    async fn unmount_shared(
        self,
        conn: &Self::SharedConn,
        mountpoint: &Path,
        options: &[MountOpt],
    ) -> Result<()>;

    async fn unmount(
        self,
        conn: &Self::Conn,
        mountpoint: &Path,
        options: &[MountOpt],
    ) -> Result<()>;
}
