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
pub use nix::unistd::{AccessFlags, Gid, Pid, Uid};

pub use access::AccessReq;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ino(NonZeroU64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Request {
    id: u64,
    uid: Uid,
    gid: Gid,
    pid: Pid,
}

impl crate::serve::Server {
    fn decode<T: bytemuck::Pod>(
        &self,
        bytes: &[u8],
        fs: &crate::async_rc::AsyncRc<impl crate::Filesystem>,
        req: &Request,
    ) -> Option<T> {
        match from_bytes::<T>(bytes) {
            Ok(value) => Some(value),
            Err(err) => {
                let mut tx = self.tx.clone();
                let unique = req.id();
                let fs = fs.clone();
                compio::runtime::spawn(async move {
                    let _fs = fs;
                    let _ = send_error(err, unique, &mut tx).await;
                })
                .detach();
                None
            }
        }
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

fn from_bytes<T: bytemuck::Pod>(bytes: &[u8]) -> crate::Result<T> {
    if bytes.len() < std::mem::size_of::<T>() {
        return Err(crate::Error::EINVAL);
    }
    let mut out = std::mem::MaybeUninit::<T>::uninit();
    unsafe {
        std::ptr::copy_nonoverlapping(
            bytes.as_ptr(),
            out.as_mut_ptr().cast(),
            std::mem::size_of::<T>(),
        );
    }
    Ok(unsafe { out.assume_init() })
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
