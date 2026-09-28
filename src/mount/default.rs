use super::{Mount, Unmount};
use crate::MountOpt;
use crate::conn::{Conn, DevFuseConn, DevFuseSharedConn};

use compio::buf::{BufResult, IoBuf, IoBufMut, IoVectoredBuf};

use std::io::Result;
use std::os::fd::AsFd;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DefaultMount;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DefaultUnmount {
    direct: bool,
}

impl Mount for DefaultMount {
    type SharedConn = DevFuseSharedConn;
    type Conn = DevFuseConn;
    type Unmount = DefaultUnmount;

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

impl Unmount for DefaultUnmount {
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
