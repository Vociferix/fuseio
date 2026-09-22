use crate::MountOpt;

use std::io::Result;
use std::os::fd::{BorrowedFd, OwnedFd};
use std::path::Path;

pub async fn mount(dev_path: &Path, mountpoint: &Path, options: &[MountOpt]) -> Result<OwnedFd> {
    todo!()
}

pub async fn unmount(dev: BorrowedFd<'_>, mountpoint: &Path, options: &[MountOpt]) -> Result<()> {
    todo!()
}
