use crate::Buf;

use std::num::NonZeroU64;

pub mod access;
pub mod batch_forget;
pub mod bmap;
pub mod copy_file_range;
pub mod create;
pub mod exchange;
pub mod fallocate;
pub mod flush;
pub mod forget;
pub mod fsync;
pub mod fsyncdir;
pub mod getattr;
pub mod getlk;
pub mod getxattr;
pub mod getxtimes;
pub mod interrupt;
pub mod ioctl;
pub mod link;
pub mod listxattr;
pub mod lookup;
pub mod lseek;
pub mod mkdir;
pub mod mknod;
pub mod notify_reply;
pub mod open;
pub mod opendir;
pub mod poll;
pub mod read;
pub mod readdir;
pub mod readdirplus;
pub mod readlink;
pub mod release;
pub mod releasedir;
pub mod removexattr;
pub mod rename;
pub mod rename2;
pub mod rmdir;
pub mod setattr;
pub mod setlk;
pub mod setvolname;
pub mod setxattr;
pub mod statfs;
pub mod symlink;
pub mod unlink;
pub mod write;

#[doc(inline)]
pub use nix::{
    fcntl::{FallocateFlags, FlockArg, OFlag},
    sys::stat::Mode,
    unistd::{AccessFlags, Gid, Pid, Uid},
};

pub use access::AccessReq;
pub use batch_forget::{ForgetIno, ForgetReq};
pub use bmap::BmapReq;
pub use copy_file_range::{CopyFileRangePos, CopyFileRangeReq};
pub use create::{CreateReq, CreatedFile};
pub use fallocate::FallocateReq;
pub use flush::FlushReq;
pub use fsync::FsyncReq;
pub use fsyncdir::FsyncDirReq;
pub use getattr::{GetAttrsReq, InodeAttrs};
pub use getlk::{FileLock, FileRange, GetLockReq, LockKind};
pub use getxattr::GetXattrReq;
pub use link::LinkReq;
pub use listxattr::GetXattrKeysReq;
pub use lookup::{Entry, LookupReq};
pub use mknod::MakeInodeReq;
pub use open::{OpenAccessMode, OpenFlags, OpenReq, OpenedFile};
pub use removexattr::RemoveXattrReq;
pub use setlk::{FlockReq, SetLockReq};
pub use setxattr::{SetXattrFlags, SetXattrReq};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ino(NonZeroU64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileHandle(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LockOwner(pub u64);

#[cfg(target_os = "macos")]
#[doc(inline)]
pub use nix::sys::stat::FileFlag;

#[cfg(not(target_os = "macos"))]
bitflags::bitflags! {
    /// File flags.
    pub struct FileFlag: u8 {
        /// The file may only be appended to.
        const SF_APPEND = 0;
        /// The file has been archived.
        const SF_ARCHIVED = 0;
        /// The file may not be changed.
        const SF_IMMUTABLE = 0;
        /// Mask of superuser changeable flags
        const SF_SETTABLE = 0;
        /// The file may only be appended to.
        const UF_APPEND = 0;
        /// File is compressed at the file system level.
        const UF_COMPRESSED = 0;
        /// The file may be hidden from directory listings at the application's
        /// discretion.
        const UF_HIDDEN = 0;
        /// The file may not be changed.
        const UF_IMMUTABLE = 0;
        /// Do not dump the file.
        const UF_NODUMP = 0;
        /// The directory is opaque when viewed through a union stack.
        const UF_OPAQUE = 0;
        /// Mask of owner changeable flags.
        const UF_SETTABLE = 0;
        /// File renames and deletes are tracked.
        const UF_TRACKED = 0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Request {
    id: u64,
    uid: Uid,
    gid: Gid,
    pid: Pid,
}

#[derive(Debug, Clone)]
pub struct Body {
    msg: Buf,
    offset: usize,
    len: usize,
}

impl Body {
    pub fn new<I>(msg: Buf, slice_range: I) -> Self
    where
        I: std::slice::SliceIndex<[u8], Output = [u8]>,
    {
        let (offset, len) = {
            let slice = &msg.as_slice()[slice_range];
            let offset = unsafe { slice.as_ptr().offset_from_unsigned(msg.as_ptr()) };
            (offset, slice.len())
        };

        Self { msg, offset, len }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn as_ptr(&self) -> *const u8 {
        unsafe { self.msg.as_ptr().add(self.offset) }
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        unsafe { self.msg.as_mut_ptr().add(self.offset) }
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.as_ptr(), self.len) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.as_mut_ptr(), self.len) }
    }

    pub fn pop_front(&mut self, n: usize) -> usize {
        let n = n.min(self.len);
        self.offset += n;
        self.len -= n;
        n
    }

    pub fn pop_back(&mut self, n: usize) -> usize {
        let n = n.min(self.len);
        self.len -= n;
        n
    }

    pub fn truncate(&mut self, new_len: usize) {
        if new_len < self.len {
            self.len = new_len;
        }
    }

    pub fn rtruncate(&mut self, new_len: usize) {
        if new_len < self.len {
            let n = self.len - new_len;
            self.offset += n;
            self.len = new_len;
        }
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }
}

impl std::borrow::Borrow<[u8]> for Body {
    fn borrow(&self) -> &[u8] {
        self.as_slice()
    }
}

impl AsRef<[u8]> for Body {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl std::ops::Deref for Body {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        self.as_slice()
    }
}

fn handle_error<E: std::error::Error>(res: Result<(), E>) {
    if let Err(err) = res {
        log::error!("failed to send response to kernel: {err}");
    }
}

async fn send_errno(
    code: i32,
    unique: u64,
    tx: &mut crate::channel::Sender,
) -> std::io::Result<()> {
    let out = crate::layout::HeaderOut {
        len: const { std::mem::size_of::<crate::layout::HeaderOut>() as u32 },
        error: -code,
        unique,
    };

    tx.send(out).await
}

async fn send_result(
    res: crate::Result<()>,
    unique: u64,
    tx: &mut crate::channel::Sender,
) -> std::io::Result<()> {
    send_errno(
        match res {
            Ok(()) => 0,
            Err(err) => err.raw_os_error(),
        },
        unique,
        tx,
    )
    .await
}

async fn send_error(
    err: crate::Error,
    unique: u64,
    tx: &mut crate::channel::Sender,
) -> std::io::Result<()> {
    send_errno(err.raw_os_error(), unique, tx).await
}

async fn send_ok(unique: u64, tx: &mut crate::channel::Sender) -> std::io::Result<()> {
    send_errno(0, unique, tx).await
}

// This function is meant to be called on the `body` argument passed to an operation
// request handler in order to decode the body or subheader.
//
// All FUSE messages have an alignment of 8 or less, and the `body` argument passed
// to operation request handlers is meant to always be aligned to 8 bytes. If the
// type `T` has an alignment larger than 8 bytes, this function will panic (but is
// written such that the compiler should optimize this panic out as long as `T` is
// aligned to 8 bytes or less).
//
// To guard against bugs and misuse, if `bytes` does not start at an address aligned
// properly for `T` or `bytes` is smaller than `T`, this function will return an
// error.
fn decode<T: bytemuck::Pod>(mut body: Body) -> crate::Result<(T, Body)> {
    assert!(std::mem::align_of::<T>() <= std::mem::align_of::<u64>());

    let ptr = body.as_ptr() as *const T;

    if body.len() < std::mem::size_of::<T>() || !ptr.is_aligned() {
        return Err(crate::Error::EINVAL);
    }

    let val = unsafe { std::ptr::read(ptr) };

    body.offset += std::mem::size_of::<T>();
    body.len -= std::mem::size_of::<T>();

    Ok((val, body))
}

impl Ino {
    pub const fn from_raw(ino: u64) -> Option<Self> {
        if let Some(ino) = NonZeroU64::new(ino) {
            Some(Self(ino))
        } else {
            None
        }
    }

    pub const unsafe fn from_raw_unchecked(ino: u64) -> Self {
        Self(unsafe { NonZeroU64::new_unchecked(ino) })
    }

    pub const fn as_raw(self) -> u64 {
        self.0.get()
    }
}

impl From<NonZeroU64> for Ino {
    fn from(ino: NonZeroU64) -> Self {
        Self(ino)
    }
}

impl From<Ino> for NonZeroU64 {
    fn from(ino: Ino) -> Self {
        ino.0
    }
}

impl std::fmt::Display for Ino {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::fmt::UpperHex for Ino {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::UpperHex::fmt(&self.0, f)
    }
}

impl std::fmt::LowerHex for Ino {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::LowerHex::fmt(&self.0, f)
    }
}

impl std::fmt::Octal for Ino {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Octal::fmt(&self.0, f)
    }
}

impl std::fmt::Binary for Ino {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Binary::fmt(&self.0, f)
    }
}

impl std::fmt::Display for FileHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::fmt::UpperHex for FileHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::UpperHex::fmt(&self.0, f)
    }
}

impl std::fmt::LowerHex for FileHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::LowerHex::fmt(&self.0, f)
    }
}

impl std::fmt::Octal for FileHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Octal::fmt(&self.0, f)
    }
}

impl std::fmt::Binary for FileHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Binary::fmt(&self.0, f)
    }
}

impl std::fmt::Display for LockOwner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::fmt::UpperHex for LockOwner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::UpperHex::fmt(&self.0, f)
    }
}

impl std::fmt::LowerHex for LockOwner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::LowerHex::fmt(&self.0, f)
    }
}

impl std::fmt::Octal for LockOwner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Octal::fmt(&self.0, f)
    }
}

impl std::fmt::Binary for LockOwner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Binary::fmt(&self.0, f)
    }
}

impl Request {
    pub(crate) fn new(hdr: &crate::layout::HeaderIn) -> Self {
        Self {
            id: hdr.unique,
            uid: hdr.uid.into(),
            gid: hdr.gid.into(),
            pid: Pid::from_raw(hdr.pid.cast_signed()),
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn uid(&self) -> Uid {
        self.uid
    }

    pub fn gid(&self) -> Gid {
        self.gid
    }

    pub fn pid(&self) -> Pid {
        self.pid
    }
}
