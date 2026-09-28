use crate::buf::IntoIoBuf;
use crate::conn::Connection;
use crate::handshake::{Config, KernelConfig};
use crate::types::PollFlags;
use crate::{Error, MountOpt, Result};

pub mod req {
    pub use crate::req::*;
}

pub mod types {
    pub use crate::types::{
        Abi, AccessFlags, CopyFileRangePos, DeviceNumber, Feature, FileFlag, FileHandle, FileRange,
        FileTime, ForgetIno, FsCaps, Gid, Ino, InodeKind, IoctlCmd, IoctlDirection, KernelCaps,
        LockKind, LockOwner, Mode, OFlag, OpenAccessMode, OpenedFlags, Pid, PollFlags, PollNotify,
        RenameMode, SFlag, StatXAttrs, StatXSync, Tgid, Uid, Version, Whence, XattrMode,
    };

    pub use crate::handshake::{Config, KernelConfig};

    pub use crate::req::{
        read_dir::{DirEntry, DirEntryBuf},
        read_dir_plus::{DirEntryPlus, DirEntryPlusBuf},
        xattr_keys::XattrKeyBuf,
        xattr_keys_len::XattrKeyLenBuf,
    };

    pub use crate::proto::response::{
        Attrs, Created, Entry, FsAttrs, IoctlReply, Opened, PosixLock, StatX, XTimes,
    };

    pub use crate::passthrough::{BackingId, PassthroughFd};

    pub use crate::notify_error::NotifyError;

    pub use crate::context::{CacheData, Context, NotifyPruneCache};

    pub use crate::conn::Connection;
}

pub mod prelude {
    pub use super::req::*;
    pub use super::types::*;
    pub use super::{BindFs, Fs, MountFs};
    pub use crate::buf::{Buf, BufPool, NoData, Pod, Vectored};
    pub use crate::{Error, Result};
}

pub trait Fs<C: Connection>: Sized + 'static {
    // TODO: document that unmounting will deadlock unless all instances of
    // `Context`, `PassthroughFd`, and `NotifyPruneCache` are dropped.
    // Specifically, these don't _necessarily_ need to be dropped before
    // `Fs::unmount` returns, but unmounting will not progress after
    // `Fs::unmount` returns until they are all dropped. Meaning, background
    // tasks may drop them some time after `Fs::unmount` returns.
    async fn unmount(self, ctx: types::Context<C>) {
        let _ = (self, ctx);
    }

    async fn lookup(&self, req: req::LookupReq<C>) -> Result<types::Entry> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn forget(&self, req: req::ForgetReq<C>) {
        let _ = (self, req);
    }

    async fn get_attrs(&self, req: req::GetAttrsReq<C>) -> Result<types::Attrs> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn set_attrs(&self, req: req::SetAttrsReq<C>) -> Result<types::Attrs> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn read_link(&self, req: req::ReadLinkReq<C>) -> Result<impl IntoIoBuf> {
        let _ = (self, req);
        Err::<crate::buf::NoData, _>(Error::ENOSYS)
    }

    async fn make_node(&self, req: req::MakeNodeReq<C>) -> Result<types::Entry> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn unlink_node(&self, req: req::UnlinkNodeReq<C>) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn make_dir(&self, req: req::MakeDirReq<C>) -> Result<types::Entry> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn remove_dir(&self, req: req::RemoveDirReq<C>) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn symlink(&self, req: req::SymlinkReq<C>) -> Result<types::Entry> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn rename(&self, req: req::RenameReq<C>) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn link(&self, req: req::LinkReq<C>) -> Result<types::Entry> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    /// Opens a file or directory.
    ///
    /// The default hands out a zero file handle and keeps no state, as libfuse
    /// does. Replying `ENOSYS` instead would fail every `open(2)` on macOS,
    /// whose protocol version predates the kernel treating that as "opening
    /// needs no reply".
    async fn open(&self, req: req::OpenReq<C>) -> Result<types::Opened> {
        let _ = (self, req);
        Ok(types::Opened::new(types::FileHandle(0)))
    }

    async fn read(&self, req: req::ReadReq<C>) -> Result<impl IntoIoBuf> {
        let _ = (self, req);
        Err::<crate::buf::NoData, _>(Error::ENOSYS)
    }

    async fn write(&self, req: req::WriteReq<C>) -> Result<usize> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn flush(&self, req: req::FlushReq<C>) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    /// Closes a file or directory opened by [`open`](Self::open).
    ///
    /// The default succeeds, to match an `open` that keeps no state.
    async fn close(&self, req: req::CloseReq<C>) -> Result<()> {
        let _ = (self, req);
        Ok(())
    }

    async fn fsync(&self, req: req::FsyncReq<C>) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn read_dir(&self, req: req::ReadDirReq<C>) -> Result<types::DirEntryBuf> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    /// Reports filesystem-wide statistics.
    ///
    /// The default reports the placeholder values libfuse uses, since `ENOSYS`
    /// would break `df` and can fail the mount on macOS, which asks during
    /// mounting.
    async fn statfs(&self, req: req::StatFsReq<C>) -> Result<types::FsAttrs> {
        let _ = (self, req);
        Ok(types::FsAttrs::new())
    }

    async fn get_xattr_len(&self, req: req::GetXattrLenReq<C>) -> Result<usize> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn get_xattr(&self, req: req::GetXattrReq<C>) -> Result<impl IntoIoBuf> {
        let _ = (self, req);
        Err::<crate::buf::NoData, _>(Error::ENOSYS)
    }

    async fn set_xattr(&self, req: req::SetXattrReq<C>) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn xattr_keys_len(&self, req: req::XattrKeysLenReq<C>) -> Result<types::XattrKeyLenBuf> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn xattr_keys(&self, req: req::XattrKeysReq<C>) -> Result<types::XattrKeyBuf> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn remove_xattr(&self, req: req::RemoveXattrReq<C>) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn access(&self, req: req::AccessReq<C>) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn create_file(&self, req: req::CreateFileReq<C>) -> Result<types::Created> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn test_posix_lock(&self, req: req::TestPosixLockReq<C>) -> Result<types::PosixLock> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn try_posix_lock(&self, req: req::PosixLockReq<C>) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn posix_lock(&self, req: req::PosixLockReq<C>) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn map_block(&self, req: req::MapBlockReq<C>) -> Result<u64> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn ioctl(&self, req: req::IoctlReq<C>) -> Result<types::IoctlReply<impl IntoIoBuf>> {
        let _ = (self, req);
        Err::<types::IoctlReply<crate::buf::NoData>, _>(Error::ENOSYS)
    }

    async fn poll(&self, req: req::PollReq<C>) -> Result<PollFlags> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn try_flock(&self, req: req::FlockReq<C>) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn flock(&self, req: req::FlockReq<C>) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn fallocate(&self, req: req::FallocateReq<C>) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn read_dir_plus(&self, req: req::ReadDirPlusReq<C>) -> Result<types::DirEntryPlusBuf> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn copy_file_range(&self, req: req::CopyFileRangeReq<C>) -> Result<u64> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn lseek(&self, req: req::LseekReq<C>) -> Result<u64> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn tmp_file(&self, req: req::TmpFileReq<C>) -> Result<types::Created> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn statx(&self, req: req::StatXReq<C>) -> Result<types::StatX> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn syncfs(&self, req: req::SyncFsReq<C>) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn get_xtimes(&self, req: req::GetXTimesReq<C>) -> Result<types::XTimes> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn set_volume_name(&self, req: req::SetVolumeNameReq<C>) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn monitor(&self, req: req::MonitorReq<C>) {
        let _ = (self, req);
    }
}

/// Seed for mounting a FUSE filesystem.
///
/// [`MountFs`] represents a constructor for a FUSE filesystem. Typically,
/// implementors are a collection of parameters to intialize the actual
/// filesystem, often presented as a builder.
pub trait MountFs<C: Connection> {
    /// The initialized, but unbound, filesystem.
    ///
    /// To support multithreaded filesystem request handling, this type must also
    /// implement [`Clone`] and [`Send`]. Clones will be sent to additional
    /// threads to be bound and serve filesystem requests.
    type Fs: BindFs<C>;

    /// Constructs a bindable filesystem.
    ///
    /// The returned `Fs` serves as an intermediate state between mounting and
    /// serving filesystem requests. Once the connection to the kernel is ready,
    /// the filesystem will be bound (via [`BindFs::bind`]) to a thread, at which
    /// point the bound filesystem will start handling requests.
    async fn mount(self, cfg: KernelConfig, options: &[MountOpt]) -> Result<Self::Fs>;
}

/// An initialized, bindable filesystem.
///
/// Types implementing [`BindFs`] represent a FUSE filesystem that has been
/// initialized, but not yet started serving filesystem requests. The filesystem
/// will be "bound" to a thread (via [`BindFs::bind`]), and the returned bound
/// filesystem ([`BindFs::BoundFs`]) will start serving requests.
///
/// Filesystems that support multithreaded request handling must provide a type
/// implementing [`BindFs`] that also implements [`Clone`] and [`Send`]. With
/// multithreaded support enabled, the [`BindFs`] implementor will be cloned, and
/// each clone will be assigned its own thread, where it will be bound. The bound
/// filesystem ([`BindFs::BoundFs`]), does not need to implement [`Clone`] or
/// [`Send`] in any case.
pub trait BindFs<C: Connection> {
    /// The bound filesystem.
    type BoundFs: Fs<C>;

    /// Return the desired FUSE configuration for the mounted filesystem.
    ///
    /// The config is typically customized according to the capabilities specified
    /// by the [`KernelConfig`] passed to [`MountFs::mount`].
    fn config(&self) -> Config;

    /// Bind the filesystem to the current thread.
    ///
    /// In multithreaded FUSE mounts (requires `Self: Clone + Send`), `self` will
    /// be one of multiple clones that have been dispatched to separate threads.
    /// Each "bound" to its own thread, and the returned `BoundFs` need not
    /// implement [`Send`] or [`Sync`], as the instance will remain local to a
    /// single thread during its lifetime.
    ///
    /// Note that in multithreaded contexts, the filesystem may be moved to a
    /// different [`compio`] runtime from the one under which it was created (i.e.
    /// the runtime under which [`MountFs::mount`] was called). This function
    /// should also handle transitioning to the new runtime, if needed.
    async fn bind(self, ctx: types::Context<C>) -> Result<Self::BoundFs>;
}
