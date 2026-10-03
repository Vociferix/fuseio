//! The BSD half of [`DirectMount`](super::DirectMount).

use crate::MountOpt;

use std::io::Result;
use std::os::fd::BorrowedFd;
use std::path::Path;

pub(super) fn attach(
    dev: BorrowedFd<'_>,
    device: &Path,
    mountpoint: &Path,
    options: &[MountOpt],
) -> Result<()> {
    let _ = (dev, device, mountpoint, options);

    todo!("the BSD direct mount is not implemented yet")
}

pub(super) fn detach(mountpoint: &Path) -> Result<()> {
    let _ = mountpoint;

    todo!("the BSD direct unmount is not implemented yet")
}
