use crate::types::PollFlags;
use crate::{Error, IntoIoBuf, MountOpt, Result};

use futures_util::Stream;

use std::marker::PhantomData;

pub struct KernelConfig;
pub struct LookupReq<'a>(&'a ());
pub struct ForgetReq<'a>(&'a ());
pub struct GetAttrsReq<'a>(&'a ());
pub struct SetAttrsReq<'a>(&'a ());
pub struct ReadLinkReq<'a>(&'a ());
pub struct MakeNodeReq<'a>(&'a ());
pub struct UnlinkNodeReq<'a>(&'a ());
pub struct MakeDirReq<'a>(&'a ());
pub struct RemoveDirReq<'a>(&'a ());
pub struct SymlinkReq<'a>(&'a ());
pub struct RenameReq<'a>(&'a ());
pub struct LinkReq<'a>(&'a ());
pub struct OpenReq<'a>(&'a ());
pub struct ReadReq<'a>(&'a ());
pub struct WriteReq<'a>(&'a ());
pub struct FlushReq<'a>(&'a ());
pub struct CloseReq<'a>(&'a ());
pub struct FsyncReq<'a>(&'a ());
pub struct ReadDirReq<'a>(&'a ());
pub struct StatFsReq<'a>(&'a ());
pub struct GetXattrLenReq<'a>(&'a ());
pub struct GetXattrReq<'a>(&'a ());
pub struct SetXattrReq<'a>(&'a ());
pub struct XattrKeysLenReq<'a>(&'a ());
pub struct XattrKeysReq<'a>(&'a ());
pub struct RemoveXattrReq<'a>(&'a ());
pub struct AccessReq<'a>(&'a ());
pub struct CreateFileReq<'a>(&'a ());
pub struct TestPosixLockReq<'a>(&'a ());
pub struct PosixLockReq<'a>(&'a ());
pub struct MapBlockReq<'a>(&'a ());
pub struct IoctlReq<'a>(&'a ());
pub struct PollReq<'a>(&'a ());
pub struct FlockReq<'a>(&'a ());
pub struct FallocateReq<'a>(&'a ());
pub struct ReadDirPlusReq<'a>(&'a ());
pub struct CopyFileRangeReq<'a>(&'a ());
pub struct LseekReq<'a>(&'a ());
pub struct TmpFileReq<'a>(&'a ());
pub struct StatXReq<'a>(&'a ());
pub struct SyncFsReq<'a>(&'a ());

pub struct ReadLinkBuf<T>(PhantomData<T>);
pub struct DirEntries<T>(PhantomData<T>);
pub struct XattrKeyBuf<T>(PhantomData<T>);
pub struct IoctlBuf<T>(PhantomData<T>);
pub struct DirPlusEntries<T>(PhantomData<T>);

pub struct Config;
pub struct Entry;
pub struct Attrs;
pub struct Opened;
pub struct FsAttrs;
pub struct Created;
pub struct PosixLock;
pub struct StatX;

pub trait Fs: Sized {
    async fn unmount(self) {
        let _ = self;
    }

    async fn lookup(&self, req: LookupReq<'_>) -> Result<Entry> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn forget(&self, req: ForgetReq<'_>) {
        let _ = (self, req);
    }

    async fn get_attrs(&self, req: GetAttrsReq<'_>) -> Result<Attrs> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn set_attrs(&self, req: SetAttrsReq<'_>) -> Result<Attrs> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn read_link(&self, req: ReadLinkReq<'_>) -> Result<ReadLinkBuf<impl IntoIoBuf>> {
        let _ = (self, req);
        Err::<ReadLinkBuf<[u8; 0]>, _>(Error::ENOSYS)
    }

    async fn make_node(&self, req: MakeNodeReq<'_>) -> Result<Entry> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn unlink_node(&self, req: UnlinkNodeReq<'_>) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn make_dir(&self, req: MakeDirReq<'_>) -> Result<Entry> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn remove_dir(&self, req: RemoveDirReq<'_>) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn symlink(&self, req: SymlinkReq<'_>) -> Result<Entry> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn rename(&self, req: RenameReq<'_>) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn link(&self, req: LinkReq<'_>) -> Result<Entry> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn open(&self, req: OpenReq<'_>) -> Result<Opened> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn read(&self, req: ReadReq<'_>) -> Result<impl IntoIoBuf> {
        let _ = (self, req);
        Err::<[u8; 0], _>(Error::ENOSYS)
    }

    async fn write(&self, req: WriteReq<'_>) -> Result<usize> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn flush(&self, req: FlushReq<'_>) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn close(&self, req: CloseReq<'_>) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn fsync(&self, req: FsyncReq<'_>) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn read_dir(&self, req: ReadDirReq<'_>) -> Result<DirEntries<impl IntoIoBuf>> {
        let _ = (self, req);
        Err::<DirEntries<[u8; 0]>, _>(Error::ENOSYS)
    }

    async fn statfs(&self, req: StatFsReq<'_>) -> Result<FsAttrs> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn get_xattr_len(&self, req: GetXattrLenReq<'_>) -> Result<usize> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn get_xattr(&self, req: GetXattrReq<'_>) -> Result<impl IntoIoBuf> {
        let _ = (self, req);
        Err::<[u8; 0], _>(Error::ENOSYS)
    }

    async fn set_xattr(&self, req: SetXattrReq<'_>) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn xattr_keys_len(&self, req: XattrKeysLenReq<'_>) -> Result<impl Stream<Item = usize>> {
        let _ = (self, req);
        Err::<futures_util::stream::Empty<usize>, _>(Error::ENOSYS)
    }

    async fn xattr_keys(&self, req: XattrKeysReq<'_>) -> Result<XattrKeyBuf<impl IntoIoBuf>> {
        let _ = (self, req);
        Err::<XattrKeyBuf<[u8; 0]>, _>(Error::ENOSYS)
    }

    async fn remove_xattr(&self, req: RemoveXattrReq<'_>) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn access(&self, req: AccessReq<'_>) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn create_file(&self, req: CreateFileReq<'_>) -> Result<Created> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn test_posix_lock(&self, req: TestPosixLockReq<'_>) -> Result<PosixLock> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn try_posix_lock(&self, req: PosixLockReq<'_>) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn posix_lock(&self, req: PosixLockReq<'_>) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn map_block(&self, req: MapBlockReq<'_>) -> Result<u64> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn ioctl(&self, req: IoctlReq<'_>) -> Result<IoctlBuf<impl IntoIoBuf>> {
        let _ = (self, req);
        Err::<IoctlBuf<[u8; 0]>, _>(Error::ENOSYS)
    }

    async fn poll(&self, req: PollReq<'_>) -> Result<PollFlags> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn try_flock(&self, req: FlockReq<'_>) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn flock(&self, req: FlockReq<'_>) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn fallocate(&self, req: FallocateReq<'_>) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn read_dir_plus(
        &self,
        req: ReadDirPlusReq<'_>,
    ) -> Result<DirPlusEntries<impl IntoIoBuf>> {
        let _ = (self, req);
        Err::<DirPlusEntries<[u8; 0]>, _>(Error::ENOSYS)
    }

    async fn copy_file_range(&self, req: CopyFileRangeReq<'_>) -> Result<u64> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn lseek(&self, req: LseekReq<'_>) -> Result<u64> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn tmpfile(&self, req: TmpFileReq<'_>) -> Result<Created> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn statx(&self, req: StatXReq<'_>) -> Result<StatX> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn syncfs(&self, req: SyncFsReq<'_>) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }
}

/// Seed for mounting a FUSE filesystem.
///
/// [`MountFs`] represents a constructor for a FUSE filesystem. Typically,
/// implementors are a collection of parameters to intialize the actual
/// filesystem, often presented as a builder.
pub trait MountFs {
    /// The initialized, but unbound, filesystem.
    ///
    /// To support multithreaded filesystem request handling, this type must also
    /// implement [`Clone`] and [`Send`]. Clones will be sent to additional
    /// threads to be bound and serve filesystem requests.
    type Fs: BindFs;

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
pub trait BindFs {
    /// The bound filesystem.
    type BoundFs: Fs;

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
    async fn bind(self) -> Result<Self::BoundFs>;
}
