use crate::types::{Gid, Ino, Pid, ReplyInitFlags, Request, Uid};
use crate::{Error, Result, buf::Buf};

mod access;
mod batch_forget;
mod bmap;
mod copy_file_range;
mod copy_file_range_64;
mod create;
mod destroy;
mod fallocate;
mod flush;
mod forget;
mod fsync;
mod fsyncdir;
mod getattr;
mod getlk;
mod getxattr;
mod getxtimes;
mod interrupt;
mod ioctl;
mod link;
mod listxattr;
mod lookup;
mod lseek;
mod mkdir;
mod mknod;
mod notify_reply;
mod open;
mod opendir;
mod poll;
mod read;
mod readdir;
mod readdirplus;
mod readlink;
mod release;
mod releasedir;
mod removexattr;
mod rename;
mod rmdir;
mod setattr;
mod setlk;
mod setlkw;
mod setvolname;
mod setxattr;
mod statfs;
mod statx;
mod symlink;
mod syncfs;
mod tmpfile;
mod unlink;
mod write;

pub use access::Access;
pub use batch_forget::BatchForget;
pub use bmap::Bmap;
pub use copy_file_range::CopyFileRange;
pub use copy_file_range_64::CopyFileRange64;
pub use create::Create;
pub use destroy::Destroy;
pub use fallocate::Fallocate;
pub use flush::Flush;
pub use forget::Forget;
pub use fsync::Fsync;
pub use fsyncdir::FsyncDir;
pub use getattr::GetAttr;
pub use getlk::GetLk;
pub use getxattr::GetXattr;
pub use getxtimes::GetXtimes;
pub use interrupt::Interrupt;
pub use ioctl::Ioctl;
pub use link::Link;
pub use listxattr::ListXattr;
pub use lookup::Lookup;
pub use lseek::Lseek;
pub use mkdir::MkDir;
pub use mknod::MkNod;
pub use notify_reply::{NotifyReply, SharedNotifyReply};
pub use open::Open;
pub use opendir::OpenDir;
pub use poll::Poll;
pub use read::Read;
pub use readdir::ReadDir;
pub use readdirplus::ReadDirPlus;
pub use readlink::ReadLink;
pub use release::Release;
pub use releasedir::ReleaseDir;
pub use removexattr::RemoveXattr;
pub use rename::Rename;
pub use rmdir::RmDir;
pub use setattr::SetAttr;
pub use setlk::SetLk;
pub use setlkw::SetLkW;
pub use setvolname::SetVolName;
pub use setxattr::SetXattr;
pub use statfs::StatFs;
pub use statx::StatX;
pub use symlink::Symlink;
pub use syncfs::SyncFs;
pub use tmpfile::TmpFile;
pub use unlink::Unlink;
pub use write::Write;

#[derive(Debug, Clone, Copy)]
pub struct Cfg {
    pub minor_ver: u32,
    /// Flags negotiated in the INIT reply.
    pub flags: ReplyInitFlags,
}

#[derive(Debug)]
pub enum Body {
    Lookup(Lookup),
    Forget(Forget),
    GetAttr(GetAttr),
    SetAttr(SetAttr),
    ReadLink(ReadLink),
    Symlink(Symlink),
    MkNod(MkNod),
    MkDir(MkDir),
    Unlink(Unlink),
    RmDir(RmDir),
    Rename(Rename),
    Link(Link),
    Open(Open),
    Read(Read),
    Write(Write),
    StatFs(StatFs),
    Release(Release),
    Fsync(Fsync),
    SetXattr(SetXattr),
    GetXattr(GetXattr),
    ListXattr(ListXattr),
    RemoveXattr(RemoveXattr),
    Flush(Flush),
    OpenDir(OpenDir),
    ReadDir(ReadDir),
    ReleaseDir(ReleaseDir),
    FsyncDir(FsyncDir),
    GetLk(GetLk),
    SetLk(SetLk),
    SetLkW(SetLkW),
    Access(Access),
    Create(Create),
    Interrupt(Interrupt),
    Bmap(Bmap),
    Destroy(Destroy),
    Ioctl(Ioctl),
    Poll(Poll),
    NotifyReply(NotifyReply),
    BatchForget(BatchForget),
    Fallocate(Fallocate),
    ReadDirPlus(ReadDirPlus),
    Lseek(Lseek),
    CopyFileRange(CopyFileRange),
    SyncFs(SyncFs),
    TmpFile(TmpFile),
    StatX(StatX),
    CopyFileRange64(CopyFileRange64),

    #[cfg(target_os = "macos")]
    SetVolName(SetVolName),
    #[cfg(target_os = "macos")]
    GetXtimes(GetXtimes),
    // TODO
    //CuseInit(CuseInit),
}

#[derive(Debug)]
pub struct AnyRequest {
    pub req: Request,
    pub body: Body,
}

#[repr(C)]
pub struct RawHeader {
    pub len: u32,
    pub opcode: Opcode,
    pub unique: u64,
    pub nodeid: u64,
    pub uid: u32,
    pub gid: u32,
    pub pid: u32,
    pub _unused: u32,
}

const HDR_LEN: usize = std::mem::size_of::<RawHeader>();

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Opcode(pub u32);

macro_rules! opcodes {
    ($($(#[$($attrs:tt)*])* $name:ident : $val:literal),* $(,)?) => {
        impl Opcode {
            $($(#[$($attrs)*])* const $name: Self = Self($val);)*
        }
    };
}

opcodes! {
    LOOKUP: 1,
    FORGET: 2,
    GETATTR: 3,
    SETATTR: 4,
    READLINK: 5,
    SYMLINK: 6,
    MKNOD: 8,
    MKDIR: 9,
    UNLINK: 10,
    RMDIR: 11,
    RENAME: 12,
    LINK: 13,
    OPEN: 14,
    READ: 15,
    WRITE: 16,
    STATFS: 17,
    RELEASE: 18,
    FSYNC: 20,
    SETXATTR: 21,
    GETXATTR: 22,
    LISTXATTR: 23,
    REMOVEXATTR: 24,
    FLUSH: 25,
    INIT: 26,
    OPENDIR: 27,
    READDIR: 28,
    RELEASEDIR: 29,
    FSYNCDIR: 30,
    GETLK: 31,
    SETLK: 32,
    SETLKW: 33,
    ACCESS: 34,
    CREATE: 35,
    INTERRUPT: 36,
    BMAP: 37,
    DESTROY: 38,
    IOCTL: 39,
    POLL: 40,
    NOTIFY_REPLY: 41,
    BATCH_FORGET: 42,
    FALLOCATE: 43,
    READDIRPLUS: 44,
    RENAME2: 45,
    LSEEK: 46,
    COPY_FILE_RANGE: 47,
    // DAX not supported
    //SETUPMAPPING: 48,
    //REMOVEMAPPING: 49,
    SYNCFS: 50,
    TMPFILE: 51,
    STATX: 52,
    COPY_FILE_RANGE_64: 53,

    #[cfg(target_os = "macos")]
    SETVOLNAME: 61,
    #[cfg(target_os = "macos")]
    GETXTIMES: 62,
    #[cfg(target_os = "macos")]
    EXCHANGE: 63,

    CUSE_INIT: 4096,
}

impl AnyRequest {
    pub fn decode(mut buf: Buf, minor_ver: u32, flags: ReplyInitFlags) -> Result<Self> {
        if buf.len() < HDR_LEN {
            return Err(Error::EPROTO);
        }

        let hdr = unsafe { std::ptr::read(buf.as_ptr() as *const RawHeader) };

        let len = hdr.len as usize;
        if len < HDR_LEN {
            return Err(Error::EPROTO);
        }

        let ino = Ino::from_raw(hdr.nodeid);

        buf.truncate(len);

        let cfg = Cfg { minor_ver, flags };

        let body = match hdr.opcode {
            Opcode::LOOKUP => Body::Lookup(Lookup::decode(buf, ino, cfg)?),
            Opcode::FORGET => Body::Forget(Forget::decode(buf, ino, cfg)?),
            Opcode::GETATTR => Body::GetAttr(GetAttr::decode(buf, ino, cfg)?),
            Opcode::SETATTR => Body::SetAttr(SetAttr::decode(buf, ino, cfg)?),
            Opcode::READLINK => Body::ReadLink(ReadLink::decode(buf, ino, cfg)?),
            Opcode::SYMLINK => Body::Symlink(Symlink::decode(buf, ino, cfg)?),
            Opcode::MKNOD => Body::MkNod(MkNod::decode(buf, ino, cfg)?),
            Opcode::MKDIR => Body::MkDir(MkDir::decode(buf, ino, cfg)?),
            Opcode::UNLINK => Body::Unlink(Unlink::decode(buf, ino, cfg)?),
            Opcode::RMDIR => Body::RmDir(RmDir::decode(buf, ino, cfg)?),
            Opcode::RENAME => Body::Rename(Rename::decode(buf, ino, cfg)?),
            Opcode::LINK => Body::Link(Link::decode(buf, ino, cfg)?),
            Opcode::OPEN => Body::Open(Open::decode(buf, ino, cfg)?),
            Opcode::READ => Body::Read(Read::decode(buf, ino, cfg)?),
            Opcode::WRITE => Body::Write(Write::decode(buf, ino, cfg)?),
            Opcode::STATFS => Body::StatFs(StatFs::decode(buf, ino, cfg)?),
            Opcode::RELEASE => Body::Release(Release::decode(buf, ino, cfg)?),
            Opcode::FSYNC => Body::Fsync(Fsync::decode(buf, ino, cfg)?),
            Opcode::SETXATTR => Body::SetXattr(SetXattr::decode(buf, ino, cfg)?),
            Opcode::GETXATTR => Body::GetXattr(GetXattr::decode(buf, ino, cfg)?),
            Opcode::LISTXATTR => Body::ListXattr(ListXattr::decode(buf, ino, cfg)?),
            Opcode::REMOVEXATTR => Body::RemoveXattr(RemoveXattr::decode(buf, ino, cfg)?),
            Opcode::FLUSH => Body::Flush(Flush::decode(buf, ino, cfg)?),
            Opcode::OPENDIR => Body::OpenDir(OpenDir::decode(buf, ino, cfg)?),
            Opcode::READDIR => Body::ReadDir(ReadDir::decode(buf, ino, cfg)?),
            Opcode::RELEASEDIR => Body::ReleaseDir(ReleaseDir::decode(buf, ino, cfg)?),
            Opcode::FSYNCDIR => Body::FsyncDir(FsyncDir::decode(buf, ino, cfg)?),
            Opcode::GETLK => Body::GetLk(GetLk::decode(buf, ino, cfg)?),
            Opcode::SETLK => Body::SetLk(SetLk::decode(buf, ino, cfg)?),
            Opcode::SETLKW => Body::SetLkW(SetLkW::decode(buf, ino, cfg)?),
            Opcode::ACCESS => Body::Access(Access::decode(buf, ino, cfg)?),
            Opcode::CREATE => Body::Create(Create::decode(buf, ino, cfg)?),
            Opcode::INTERRUPT => Body::Interrupt(Interrupt::decode(buf, ino, cfg)?),
            Opcode::BMAP => Body::Bmap(Bmap::decode(buf, ino, cfg)?),
            Opcode::DESTROY => Body::Destroy(Destroy::decode(buf, ino, cfg)?),
            Opcode::IOCTL => Body::Ioctl(Ioctl::decode(buf, ino, cfg)?),
            Opcode::POLL => Body::Poll(Poll::decode(buf, ino, cfg)?),
            Opcode::NOTIFY_REPLY => Body::NotifyReply(NotifyReply::decode(buf, ino, cfg)?),
            Opcode::BATCH_FORGET => Body::BatchForget(BatchForget::decode(buf, ino, cfg)?),
            Opcode::FALLOCATE => Body::Fallocate(Fallocate::decode(buf, ino, cfg)?),
            Opcode::READDIRPLUS => Body::ReadDirPlus(ReadDirPlus::decode(buf, ino, cfg)?),
            Opcode::RENAME2 => Body::Rename(Rename::decode_rename2(buf, ino, cfg)?),
            Opcode::LSEEK => Body::Lseek(Lseek::decode(buf, ino, cfg)?),
            Opcode::COPY_FILE_RANGE => Body::CopyFileRange(CopyFileRange::decode(buf, ino, cfg)?),
            Opcode::SYNCFS => Body::SyncFs(SyncFs::decode(buf, ino, cfg)?),
            Opcode::TMPFILE => Body::TmpFile(TmpFile::decode(buf, ino, cfg)?),
            Opcode::STATX => Body::StatX(StatX::decode(buf, ino, cfg)?),
            Opcode::COPY_FILE_RANGE_64 => {
                Body::CopyFileRange64(CopyFileRange64::decode(buf, ino, cfg)?)
            }

            #[cfg(target_os = "macos")]
            Opcode::SETVOLNAME => Body::SetVolName(SetVolName::decode(buf, ino, cfg)?),
            #[cfg(target_os = "macos")]
            Opcode::GETXTIMES => Body::GetXtimes(GetXtimes::decode(buf, ino, cfg)?),
            #[cfg(target_os = "macos")]
            Opcode::EXCHANGE => Body::Rename(Rename::decode_exchange(buf, ino, cfg)?),

            Opcode::INIT => return Err(Error::EPROTO),
            _ => return Err(Error::ENOSYS),
        };

        Ok(Self {
            req: Request {
                id: hdr.unique,
                uid: Uid::from_raw(hdr.uid),
                gid: Gid::from_raw(hdr.gid),
                pid: Pid::from_raw(hdr.pid.cast_signed()),
            },
            body,
        })
    }
}
