#![allow(async_fn_in_trait)]

use crate::request::{
    AccessReq, BmapReq, CopyFileRangeReq, Entry, ForgetReq, GetAttrsReq, GetAttrsResp, LookupReq,
    OpenReq, OpenResp,
};
use crate::{Error, FsConfig, KernelConfig, MountOpt, Result};

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
}
