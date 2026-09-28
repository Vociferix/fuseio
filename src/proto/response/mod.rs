use super::Cfg;
use crate::{Result, buf::IntoIoBuf};

use compio::buf::IoBuf;

mod attr;
mod bmap;
mod copy_file_range;
mod create;
mod data;
mod empty;
mod entry;
mod error;
mod ioctl;
mod lseek;
mod open;
mod poll;
mod posix_lock;
//mod readlink; // use Data
mod statfs;
mod statx;
mod write;
mod xattr;
mod xtimes;

pub use attr::{Attrs, InodeAttrs};
pub use bmap::Bmap;
pub use copy_file_range::CopyFileRange;
pub use create::Created;
pub use data::Data;
pub use entry::Entry;
pub use ioctl::IoctlReply;
pub use lseek::Lseek;
pub use open::Opened;
pub use poll::Poll;
pub use posix_lock::PosixLock;
pub use statfs::FsAttrs;
pub use statx::StatX;
pub use write::Write;
pub use xattr::XattrLen;
pub use xtimes::XTimes;

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

impl<T> EncodeResp for Result<T>
where
    T: EncodeResp,
    crate::Error: From<T::Error>,
{
    type Error = std::convert::Infallible;

    fn encode(self, id: u64, cfg: Cfg) -> std::result::Result<impl IntoIoBuf, Self::Error> {
        match self {
            Ok(resp) => match resp.encode(id, cfg) {
                Ok(buf) => Ok(buf.left_buf()),
                Err(err) => {
                    let Ok(buf) = crate::Error::from(err).encode(id, cfg);
                    Ok(buf.right_buf())
                }
            },
            Err(err) => {
                let Ok(buf) = err.encode(id, cfg);
                Ok(buf.right_buf())
            }
        }
    }
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
