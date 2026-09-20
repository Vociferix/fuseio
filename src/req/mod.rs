use crate::Result;
use crate::buf::{BufPool, IntoIoBuf};
use crate::context::{CacheData, Context};
use crate::passthrough::PassthroughFd;
use crate::types::{FileRange, Gid, Ino, Pid, Request, Uid, Version};

use std::ffi::OsStr;

mod access;
mod close;
mod copy_file_range;
mod create_file;
mod fallocate;
mod flock;
mod flush;
mod forget;
mod fsync;
mod get_attrs;
mod get_xattr;
mod get_xattr_len;
mod ioctl;
mod link;
mod lookup;
mod lseek;
mod make_dir;
mod make_node;
mod map_block;
mod open;
pub(crate) mod poll;
mod posix_lock;
mod read;
mod read_dir;
mod read_dir_plus;
mod read_link;
mod remove_dir;
mod remove_xattr;
mod rename;
mod set_attrs;
mod set_xattr;
mod statfs;
mod statx;
mod symlink;
mod syncfs;
mod test_posix_lock;
mod tmp_file;
mod unlink_node;
mod write;
mod xattr_keys;
mod xattr_keys_len;

pub use access::AccessReq;
pub use close::CloseReq;
pub use copy_file_range::CopyFileRangeReq;
pub use create_file::CreateFileReq;
pub use fallocate::FallocateReq;
pub use flock::FlockReq;
pub use flush::FlushReq;
pub use forget::ForgetReq;
pub use fsync::FsyncReq;
pub use get_attrs::GetAttrsReq;
pub use get_xattr::GetXattrReq;
pub use get_xattr_len::GetXattrLenReq;
pub use ioctl::IoctlReq;
pub use link::LinkReq;
pub use lookup::LookupReq;
pub use lseek::LseekReq;
pub use make_dir::MakeDirReq;
pub use make_node::MakeNodeReq;
pub use map_block::MapBlockReq;
pub use open::OpenReq;
pub use poll::PollReq;
pub use posix_lock::PosixLockReq;
pub use read::ReadReq;
pub use read_dir::{DirEntry, DirEntryBuf, ReadDirReq};
pub use read_dir_plus::{DirEntryPlus, DirEntryPlusBuf, ReadDirPlusReq};
pub use read_link::ReadLinkReq;
pub use remove_dir::RemoveDirReq;
pub use remove_xattr::RemoveXattrReq;
pub use rename::RenameReq;
pub use set_attrs::SetAttrsReq;
pub use set_xattr::SetXattrReq;
pub use statfs::StatFsReq;
pub use statx::StatXReq;
pub use symlink::SymlinkReq;
pub use syncfs::SyncFsReq;
pub use test_posix_lock::TestPosixLockReq;
pub use tmp_file::TmpFileReq;
pub use unlink_node::UnlinkNodeReq;
pub use write::WriteReq;
pub use xattr_keys::{XattrKeyBuf, XattrKeysReq};
pub use xattr_keys_len::XattrKeysLenReq;

use read_dir::RawDirEntry;

#[derive(Clone)]
pub struct Req {
    ctx: Context,
    req: Request,
}

impl Req {
    pub(crate) fn new<F, U>(server: &crate::server::Server<F, U>, req: Request) -> Self {
        Self {
            ctx: Context::new(server.inner.clone()),
            req,
        }
    }

    pub fn context(&self) -> &Context {
        &self.ctx
    }

    pub fn id(&self) -> u64 {
        self.req.id()
    }

    pub fn uid(&self) -> Uid {
        self.req.uid()
    }

    pub fn gid(&self) -> Gid {
        self.req.gid()
    }

    pub fn pid(&self) -> Pid {
        self.req.pid()
    }
}

impl std::fmt::Debug for Req {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.req, f)
    }
}

impl std::ops::Deref for Req {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}
