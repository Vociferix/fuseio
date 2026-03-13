use super::{Mount, Unmount};
use crate::MountOpt;

use std::borrow::Cow;
use std::io::Result;
use std::os::fd::BorrowedFd;
use std::path::Path;

/// A [`Mount`] implementation that calls libfuse's `fusermount` executable.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Fusermount {
    cmd: Option<Cow<'static, Path>>,
}

/// The unmount half of [`Fusermount`].
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FusermountUnmount {
    cmd: Cow<'static, Path>,
}

impl Fusermount {
    pub const fn from_static(path: &'static str) -> Self {
        // SAFETY: On UNIX-like platforms, `Path` is just an arbitrary
        //         byte slice, and `str` is represented as a byte slice
        //         on all platforms. This crate only supports UNIX-like
        //         platforms, so this is always safe. If we support
        //         Windows somehow in the future, this will need to
        //         change.
        let path: &'static Path = unsafe { std::mem::transmute(path) };

        Self::from_static_path(path)
    }

    pub const fn from_static_path(path: &'static Path) -> Self {
        Self {
            cmd: Some(Cow::Borrowed(path)),
        }
    }

    /// Constructs a [`Fusermount`] mounter that will use the provided `fusermount` executable.
    ///
    /// If the provided path does not exist, or is not an excutable compatible with `fusermount`,
    /// an error will be returned when [`Mount::mount`] is called.
    pub fn from_path<P>(path: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            cmd: Some(path.as_ref().to_path_buf().into()),
        }
    }

    /// Constructs a [`Fusermount`] mounter that will autodetect the `fusermount` executable.
    ///
    /// When mounting, this mounter will search common default system locations for the
    /// executable. If the executable is not found, it will then search other directories
    /// according to the `PATH` environment variable. If the executable is still not found,
    /// mounting results in an error. If this is not suitable, use [`Self::from_path`] to
    /// specify the executable location.
    pub const fn new() -> Self {
        Self { cmd: None }
    }
}

impl<P: AsRef<Path>> From<P> for Fusermount {
    fn from(path: P) -> Self {
        Self::from_path(path)
    }
}

impl Mount for Fusermount {
    type Unmount = FusermountUnmount;

    async fn mount(
        &self,
        dev: BorrowedFd<'_>,
        mount: &Path,
        options: &[MountOpt],
    ) -> Result<Self::Unmount> {
        todo!()
    }
}

impl Unmount for FusermountUnmount {
    async fn unmount(self, dev: BorrowedFd<'_>, mount: &Path, options: &[MountOpt]) -> Result<()> {
        todo!()
    }
}
