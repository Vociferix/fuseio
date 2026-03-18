#![allow(async_fn_in_trait)]

use crate::request::{
    AccessReq, BmapReq, CopyFileRangeReq, CreateReq, CreateResp, Entry, FallocateReq, FileLock,
    FlockReq, FlushReq, ForgetReq, FsyncDirReq, FsyncReq, GetAttrsReq, GetAttrsResp, GetLockReq,
    GetXattrLenReq, GetXattrReq, LookupReq, MknodReq, OpenReq, OpenResp, SetLockReq,
};
use crate::{Error, FsConfig, KernelConfig, MountOpt, Result};

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

    async fn access(&self, req: &AccessReq) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn forget(&self, req: &ForgetReq<'_>) {
        let _ = (self, req);
    }

    async fn bmap(&self, req: &BmapReq) -> Result<u64> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn copy_file_range(&self, req: &CopyFileRangeReq) -> Result<u64> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn get_attrs(&self, req: &GetAttrsReq) -> Result<GetAttrsResp> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn open(&self, req: &OpenReq) -> Result<OpenResp> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn lookup(&self, req: &LookupReq<'_>) -> Result<Entry> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn mknod(&self, req: &MknodReq<'_>) -> Result<Entry> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn create(&self, req: &CreateReq<'_>) -> Result<CreateResp> {
        let mknod_req = MknodReq {
            req: req.req,
            parent: req.parent,
            mode: req.mode,
            umask: req.umask,
            rdev: 0,
            name: req.name,
        };

        let entry = self.mknod(&mknod_req).await?;

        let open_req = OpenReq {
            req: req.req,
            ino: entry.ino,
            flags: req.flags,
            dev: req.dev.clone(),
        };

        let open_resp = self.open(&open_req).await?;

        Ok(CreateResp::from_parts(entry, open_resp))
    }

    async fn fallocate(&self, req: &FallocateReq) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn flush(&self, req: &FlushReq) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn fsync(&self, req: &FsyncReq) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn fsyncdir(&self, req: &FsyncDirReq) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn getlock(&self, req: &GetLockReq) -> Result<FileLock> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn setlock(&self, req: &SetLockReq) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn flock(&self, req: &FlockReq) -> Result<()> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn getxattrlen(&self, req: &GetXattrLenReq<'_>) -> Result<usize> {
        let _ = (self, req);
        Err(Error::ENOSYS)
    }

    async fn getxattr(&self, req: &GetXattrReq<'_>) -> Result<impl IoBuf> {
        let _ = (self, req);
        Err::<crate::Buf, _>(Error::ENOSYS)
    }
}
