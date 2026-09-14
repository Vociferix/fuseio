use super::Cfg;
use crate::{IntoIoBuf, Result};

use compio::buf::IoBuf;

mod attr;
mod bmap;
mod copy_file_range;
mod create;
mod data;
mod empty;
mod entry;
mod error;
mod flock;
mod ioctl;
mod ioctl_retry;
mod lseek;
mod open;
mod poll;
mod posix_lock;
//mod readlink; // use Data
mod statfs;
mod statx;
mod write;
mod xattr;

pub use attr::{Attr, InodeAttrs};
pub use bmap::Bmap;
pub use copy_file_range::CopyFileRange;
pub use create::Create;
pub use data::Data;
pub use empty::Empty;
pub use entry::Entry;
pub use flock::Flock;
pub use ioctl::Ioctl;
pub use ioctl_retry::{IoctlLookup, IoctlRetry};
pub use lseek::Lseek;
pub use open::Open;
pub use posix_lock::PosixLock;
pub use statfs::StatFs;
pub use statx::StatX;
pub use write::Write;
pub use xattr::XattrLen;

#[repr(C)]
#[derive(Debug, Default)]
struct RawHeader {
    len: u32,
    err: i32,
    id: u64,
}

pub trait EncodeResp
where
    crate::Error: From<Self::Error>,
{
    type Error;

    fn encode(self, id: u64, cfg: Cfg) -> std::result::Result<impl IntoIoBuf, Self::Error>;
}

impl IoBuf for RawHeader {
    fn as_init(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.buf_ptr(), self.buf_len()) }
    }

    fn buf_len(&self) -> usize {
        const { std::mem::size_of::<RawHeader>() }
    }

    fn buf_ptr(&self) -> *const u8 {
        self as *const Self as *const u8
    }
}
