use crate::MountOpt;

use std::io::Result;
use std::os::fd::BorrowedFd;
use std::path::Path;

pub async fn mount(dev: BorrowedFd<'_>, mountpoint: &Path, options: &[MountOpt]) -> Result<()> {
    todo!()
}

pub async fn unmount(dev: BorrowedFd<'_>, mountpoint: &Path, options: &[MountOpt]) -> Result<()> {
    todo!()
}
