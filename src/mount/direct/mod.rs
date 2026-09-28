use super::{Mount, Unmount};
use crate::MountOpt;
use crate::conn::{Conn, DevFuseConn, DevFuseSharedConn};

use std::io::Result;
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
        Ok((todo!(), std::future::ready(todo!())))
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
        todo!()
    }
}
