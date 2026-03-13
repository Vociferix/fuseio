use crate::request::AccessReq;
use crate::{Error, FsConfig, KernelConfig, Result};

pub trait Filesystem: 'static {
    async fn initialize(&mut self, kconf: KernelConfig) -> Result<FsConfig> {
        Ok(kconf.into())
    }

    async fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }

    /*
    async fn lookup(&self, req: LookupReq<'_>) -> Result<Entry> {
        let _ = req;
        Err(Error::ENOSYS)
    }

    async fn forget(&self, req: ForgetReq<'_>) {
        let _ = req;
    }

    async fn getattr(&self, req: GetAttrReq<'_>) -> Result<InodeAttrs> {
        let _ = req;
        Err(Error::ENOSYS)
    }

    async fn setattr(&self, req: SetAttrReq<'_>) -> Result<InodeAttrs> {
        let _ = req;
        Err(Error::ENOSYS)
    }

    async fn readlink(&self, req: ReadLinkReq<'_>) -> Result<()> {
        let _ = req;
        Err(Error::ENOSYS)
    }

    async fn mknod(&self, req: MkNodeReq<'_>) -> Result<Entry> {
        let _ = req;
        Err(Error::ENOSYS)
    }

    async fn mkdir(&self, req: MkDirReq<'_>) -> Result<Entry> {
        let _ = req;
        Err(Error::ENOSYS)
    }

    async fn unlink(&self, req: UnlinkReq<'_>) -> Result<()> {
        let _ = req;
        Err(Error::ENOSYS)
    }

    async fn rmdir(&self, req: RmDirReq<'_>) -> Result<()> {
        let _ = req;
        Err(Error::ENOSYS)
    }

    async fn symlink(&self, req: SymlinkReq<'_>) -> Result<Entry> {
        let _ = req;
        Err(Error::ENOSYS)
    }

    async fn rename(&self, req: RenameReq<'_>) -> Result<()> {
        let _ = req;
        Err(Error::ENOSYS)
    }

    async fn link(&self, req: LinkReq<'_>) -> Result<Entry> {
        let _ = req;
        Err(Error::ENOSYS)
    }

    async fn open(&self, req: OpenReq<'_>) -> Result<OpenedFile> {
        let _ = req;
        Err(Error::ENOSYS)
    }

    async fn read(&self, req: ReadReq<'_>) -> Result<()> {
        let _ = req;
        Err(Error::ENOSYS)
    }

    async fn write(&self, req: WriteReq<'_>) -> Result<usize> {
        let _ = req;
        Err(Error::ENOSYS)
    }

    async fn flush(&self, req: FlushReq<'_>) -> Result<()> {
        let _ = req;
        Ok(())
    }

    async fn release(&self, req: ReleaseReq<'_>) -> Result<()> {
        let _ = req;
        Ok(())
    }

    async fn fsync(&self, req: FsyncReq<'_>) -> Result<()> {
        let _ = req;
        Ok(())
    }

    async fn opendir(&self, req: OpenDirReq<'_>) -> Result<OpenedDir> {
        let _ = req;
        Err(Error::ENOSYS)
    }

    async fn readdir(&self, req: ReadDirReq<'_>) -> Result<()> {
        let _ = req;
        Err(Error::ENOSYS)
    }

    async fn readdirplus(&self, req: ReadDirPlusReq<'_>) -> Result<()> {
        let _ = req;
        Err(Error::ENOSYS)
    }

    async fn releasedir(&self, req: ReleaseDirReq<'_>) -> Result<()> {
        let _ = req;
        Ok(())
    }

    async fn fsyncdir(&self, req: FsyncDirReq<'_>) -> Result<()> {
        let _ = req;
        Ok(())
    }

    async fn statfs(&self, req: StatFsReq<'_>) -> Result<StatFs> {
        let _ = req;
        Err(Error::ENOSYS)
    }

    async fn setxattr(&self, req: SetXattrReq<'_>) -> Result<()> {
        let _ = req;
        Err(Error::ENOSYS)
    }

    async fn getxattr(&self, req: GetXattrReq<'_>) -> Result<Xattr> {
        let _ = req;
        Err(Error::ENOSYS)
    }

    async fn listxattr(&self, req: ListXattrReq<'_>) -> Result<()> {
        let _ = req;
        Err(Error::ENOSYS)
    }

    async fn removexattr(&self, req: RemoveXattrReq<'_>) -> Result<()> {
        let _ = req;
        Err(Error::ENOSYS)
    }
    */

    async fn access(&self, req: &AccessReq) -> Result<()> {
        let _ = req;
        Err(Error::ENOSYS)
    }

    /*
    async fn create(&self, req: CreateReq<'_>) -> Result<CreatedFile> {
        let _ = req;
        Err(Error::ENOSYS)
    }

    async fn getlock(&self, req: GetLockReq<'_>) -> Result<Locked> {
        let _ = req;
        Err(Error::ENOSYS)
    }

    async fn setlock(&self, req: SetLockReq<'_>) -> Result<()> {
        let _ = req;
        Err(Error::ENOSYS)
    }

    async fn bmap(&self, req: BmapReq<'_>) -> Result<u64> {
        let _ = req;
        Err(Error::ENOSYS)
    }

    async fn ioctl(&self, req: IoctlReq<'_>) -> Result<Result<()>> {
        let _ = req;
        Err(Error::ENOSYS)
    }

    async fn poll(&self, req: PollReq<'_>) -> Result<PollStatus> {
        let _ = req;
        Err(Error::ENOSYS)
    }

    async fn fallocate(&self, req: FallocateReq<'_>) -> Result<()> {
        let _ = req;
        Err(Error::ENOSYS)
    }

    async fn lseek(&self, req: LseekReq<'_>) -> Result<u64> {
        let _ = req;
        Err(Error::ENOSYS)
    }

    async fn copy_file_range(&self, req: CopyFileRangeReq<'_>) -> Result<usize> {
        let _ = req;
        Err(Error::ENOSYS)
    }
    */
}
