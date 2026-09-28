use super::Cfg;
use crate::buf::IntoIoBuf;

use compio::buf::IoBuf;

mod delete;
mod entry;
mod expire_entry;
mod increment_epoch;
mod inval_entry;
mod inval_inode;
mod poll;
mod prune;
mod retrieve;
mod store;

pub use delete::Delete;
pub use expire_entry::ExpireEntry;
pub use increment_epoch::IncrementEpoch;
pub use inval_entry::InvalEntry;
pub use inval_inode::InvalInode;
pub use poll::Poll;
pub use prune::Prune;
pub use retrieve::Retrieve;
pub use store::Store;

pub trait EncodeNotify
where
    crate::Error: From<Self::Error>,
{
    type Error;

    fn encode(self, cfg: Cfg) -> Result<impl IntoIoBuf, Self::Error>;
}

#[repr(C)]
#[derive(Debug)]
struct RawHeader {
    len: u32,
    code: NotifyCode,
    _zero: u64, //< must always be zero
}

impl IoBuf for RawHeader {
    fn as_init(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.buf_ptr(), self.buf_len()) }
    }

    fn buf_ptr(&self) -> *const u8 {
        self as *const Self as *const u8
    }

    fn buf_len(&self) -> usize {
        const { std::mem::size_of::<RawHeader>() }
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct NotifyCode(u32);

impl NotifyCode {
    const POLL: Self = Self(1);

    const INVAL_INODE: Self = Self(2);

    const INVAL_ENTRY: Self = Self(3);

    const STORE: Self = Self(4);

    const RETRIEVE: Self = Self(5);

    const DELETE: Self = Self(6);

    // RESEND is unused (sort of). There may or may not be support for
    // it in the kernel, but libfuse does not implement it. This notification
    // asks for an operation to be retried, but there are other, better ways
    // to handle scenarios where this would be useful, such as simply returning
    // an `EAGAIN` error.
    #[allow(dead_code)]
    const RESEND: Self = Self(7);

    const INC_EPOCH: Self = Self(8);

    const PRUNE: Self = Self(9);
}
