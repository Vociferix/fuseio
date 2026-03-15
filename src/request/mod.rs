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
pub mod setlkw;
pub mod setvolname;
pub mod setxattr;
pub mod statfs;
pub mod symlink;
pub mod unlink;
pub mod write;

#[doc(inline)]
pub use nix::{
    fcntl::OFlag,
    sys::stat::Mode,
    unistd::{AccessFlags, Gid, Pid, Uid},
};

pub use access::AccessReq;
pub use batch_forget::{ForgetIno, ForgetReq};
pub use bmap::BmapReq;
pub use copy_file_range::{CopyFileRangePos, CopyFileRangeReq};
pub use getattr::{GetAttrsReq, GetAttrsResp, InodeAttrs};
pub use lookup::{Entry, LookupReq};
pub use open::{OpenAccessMode, OpenFlags, OpenReq, OpenResp};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ino(NonZeroU64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileHandle(u64);

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
fn decode<T: bytemuck::Pod>(bytes: &[u8]) -> crate::Result<(T, &[u8])> {
    assert!(std::mem::align_of::<T>() <= std::mem::align_of::<u64>());

    let ptr = bytes.as_ptr() as *const T;

    if bytes.len() < std::mem::size_of::<T>() || !ptr.is_aligned() {
        return Err(crate::Error::EINVAL);
    }

    Ok((unsafe { std::ptr::read(ptr) }, unsafe {
        std::slice::from_raw_parts(ptr.add(1).cast(), bytes.len() - std::mem::size_of::<T>())
    }))
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

impl FileHandle {
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn as_raw(self) -> u64 {
        self.0
    }
}

impl From<FileHandle> for u64 {
    fn from(fh: FileHandle) -> Self {
        fh.0
    }
}

impl From<u64> for FileHandle {
    fn from(raw: u64) -> FileHandle {
        FileHandle(raw)
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
