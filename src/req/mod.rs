use crate::cancel_token::CancelToken;
use crate::context::Context;
use crate::server::Server;
use crate::types::{Gid, Pid, Request, Uid};

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
mod get_xtimes;
pub(crate) mod ioctl;
mod link;
mod lookup;
mod lseek;
mod make_dir;
mod make_node;
mod map_block;
mod monitor;
mod open;
pub(crate) mod poll;
mod posix_lock;
mod read;
pub(crate) mod read_dir;
pub(crate) mod read_dir_plus;
mod read_link;
mod remove_dir;
mod remove_xattr;
mod rename;
mod set_attrs;
mod set_volume_name;
mod set_xattr;
mod statfs;
mod statx;
mod symlink;
mod syncfs;
mod test_posix_lock;
mod tmp_file;
mod unlink_node;
mod write;
pub(crate) mod xattr_keys;
pub(crate) mod xattr_keys_len;

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
pub use get_xtimes::GetXTimesReq;
pub use ioctl::IoctlReq;
pub use link::LinkReq;
pub use lookup::LookupReq;
pub use lseek::LseekReq;
pub use make_dir::MakeDirReq;
pub use make_node::MakeNodeReq;
pub use map_block::MapBlockReq;
pub use monitor::MonitorReq;
pub use open::OpenReq;
pub use poll::PollReq;
pub use posix_lock::PosixLockReq;
pub use read::ReadReq;
pub use read_dir::ReadDirReq;
pub use read_dir_plus::ReadDirPlusReq;
pub use read_link::ReadLinkReq;
pub use remove_dir::RemoveDirReq;
pub use remove_xattr::RemoveXattrReq;
pub use rename::RenameReq;
pub use set_attrs::SetAttrsReq;
pub use set_volume_name::SetVolumeNameReq;
pub use set_xattr::SetXattrReq;
pub use statfs::StatFsReq;
pub use statx::StatXReq;
pub use symlink::SymlinkReq;
pub use syncfs::SyncFsReq;
pub use test_posix_lock::TestPosixLockReq;
pub use tmp_file::TmpFileReq;
pub use unlink_node::UnlinkNodeReq;
pub use write::WriteReq;
pub use xattr_keys::XattrKeysReq;
pub use xattr_keys_len::XattrKeysLenReq;

#[derive(Clone)]
pub struct Req<C> {
    ctx: Context<C>,
    token: CancelToken,
    req: Request,
}

impl<C> Req<C> {
    pub(crate) fn new<F, U>(server: &Server<F, U>, token: CancelToken, req: Request) -> Self
    where
        U: crate::mount::Unmount<Conn = C>,
    {
        Self {
            ctx: Context::new(server.inner.clone()),
            token,
            req,
        }
    }

    pub fn context(&self) -> &Context<C> {
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

    /// The caller's process, as the kernel reports it.
    ///
    /// Linux sends the calling *thread's* id here, while the BSDs and macOS send
    /// the process id, so this is not comparable with the
    /// [`Tgid`](crate::types::Tgid) a lock request carries.
    pub fn pid(&self) -> Pid {
        self.req.pid()
    }

    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }

    pub fn cancel(&self) {
        self.token.cancel();
    }

    pub async fn cancelled(&self) {
        self.token.cancelled().await
    }

    pub async fn run_until_cancelled<F>(&self, future: F) -> Option<F::Output>
    where
        F: Future,
    {
        self.token.run_until_cancelled(future).await
    }
}

impl<C> std::fmt::Debug for Req<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.req, f)
    }
}

impl<C> std::ops::Deref for Req<C> {
    type Target = Context<C>;

    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}
