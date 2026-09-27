use super::{Mount, Unmount};
use crate::MountOpt;
use crate::conn::{DevFuseConn, DevFuseSharedConn};

use compio::buf::{BufResult, IoBuf, IoBufMut, IoVectoredBuf};

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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DirectMount {
    dev_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DirectUnmount {
    _priv: (),
}

#[derive(Debug)]
pub struct SharedDirectConn {
    _priv: (),
}

#[derive(Debug)]
pub struct DirectConn {
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
        conn: &Self::Conn,
        mountpoint: &Path,
        options: &[MountOpt],
    ) -> Result<()> {
        todo!()
    }

    async fn unmount_shared(
        self,
        conn: &Self::SharedConn,
        mountpoint: &Path,
        options: &[MountOpt],
    ) -> Result<()> {
        todo!()
    }
}
