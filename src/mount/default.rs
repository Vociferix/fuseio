use super::{Mount, Unmount};
use crate::MountOpt;

use std::io::Result;
use std::os::fd::BorrowedFd;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DefaultMount;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DefaultUnmount {
    direct: bool,
}

impl Mount for DefaultMount {
    type Unmount = DefaultUnmount;

    async fn mount(
        &self,
        dev: BorrowedFd<'_>,
        mount: &Path,
        options: &[MountOpt],
    ) -> Result<Self::Unmount> {
        todo!()
    }
}

impl Unmount for DefaultUnmount {
    async fn unmount(self, dev: BorrowedFd<'_>, mount: &Path, options: &[MountOpt]) -> Result<()> {
        todo!()
    }
}
