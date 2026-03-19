#![allow(async_fn_in_trait)]

use crate::request::{
    AccessReq, BmapReq, CopyFileRangeReq, CreateReq, CreatedFile, Entry, FallocateReq, FileLock,
    FlockReq, FlushReq, ForgetReq, FsyncDirReq, FsyncReq, GetAttrsReq, GetLockReq, GetXattrLenReq,
    GetXattrReq, InodeAttrs, LookupReq, MakeInodeReq, OpenReq, OpenedFile, SetLockReq,
};
use crate::{Error, FsConfig, IntoIoBuf, KernelConfig, MountOpt, Result};

use compio::buf::IoBuf;

pub trait Filesystem: 'static {
    async fn initialize(&mut self, kconf: KernelConfig, opts: &[MountOpt]) -> Result<FsConfig> {
        let _ = (self, opts);
        Ok(kconf.into())
    }

    async fn shutdown(&mut self) -> Result<()> {
        let _ = self;
        Ok(())
    }

    async fn access(&self, req: AccessReq) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn forget(&self, req: ForgetReq<'_>) {
        let _ = (self, req);
    }

    async fn bmap(&self, req: BmapReq) -> Result<u64> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn copy_file_range(&self, req: CopyFileRangeReq) -> Result<u64> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn get_attrs(&self, req: GetAttrsReq) -> Result<InodeAttrs> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn open(&self, req: OpenReq) -> Result<OpenedFile> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn lookup(&self, req: LookupReq<'_>) -> Result<Entry> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn make_inode(&self, req: MakeInodeReq<'_>) -> Result<Entry> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn create(&self, req: CreateReq<'_>) -> Result<CreatedFile> {
        let entry = self.make_inode(req.as_make_inode_req()).await?;
        let opened = self.open(req.as_open_req(&entry)).await?;
        Ok(CreatedFile::from_parts(entry, opened))
    }

    async fn fallocate(&self, req: FallocateReq) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn flush(&self, req: FlushReq) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn fsync(&self, req: FsyncReq) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn fsyncdir(&self, req: FsyncDirReq) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn get_lock(&self, req: GetLockReq) -> Result<FileLock> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn set_lock(&self, req: SetLockReq) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn flock(&self, req: FlockReq) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn get_xattr_len(&self, req: GetXattrLenReq<'_>) -> Result<usize> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn get_xattr(&self, req: GetXattrReq<'_>) -> Result<impl IntoIoBuf> {
        let _ = (self, req);
        Err::<crate::Buf, _>(Error::ENOSYS)
    }
}
