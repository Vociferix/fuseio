use super::Cfg;
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
mod monitor;
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
pub use monitor::Monitor;
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
    SetVolName(SetVolName),
    GetXtimes(GetXtimes),
    Monitor(Monitor),
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

#[cfg(test)]
mod header_tests {
    use super::RawHeader;

    use std::mem::{offset_of, size_of};

    // `RawHeader` is read straight off the wire, so every field has to sit
    // where `struct fuse_in_header` puts it. The `uid` in particular is what
    // decides whether a request is served at all under `allow_root`, and
    // reading it from the wrong place would be invisible until someone else's
    // request arrived.
    #[test]
    fn the_header_matches_fuse_in_header() {
        assert_eq!(offset_of!(RawHeader, len), 0);
        assert_eq!(offset_of!(RawHeader, opcode), 4);
        assert_eq!(offset_of!(RawHeader, unique), 8);
        assert_eq!(offset_of!(RawHeader, nodeid), 16);
        assert_eq!(offset_of!(RawHeader, uid), 24);
        assert_eq!(offset_of!(RawHeader, gid), 28);
        assert_eq!(offset_of!(RawHeader, pid), 32);

        // `total_extlen` and `padding`, two `u16`s this crate has no use for.
        assert_eq!(offset_of!(RawHeader, _unused), 36);
        assert_eq!(size_of::<RawHeader>(), 40);
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Opcode(pub u32);

macro_rules! opcodes {
    ($($(#[$($attrs:tt)*])* $name:ident : $val:literal),* $(,)?) => {
        impl Opcode {
            $($(#[$($attrs)*])* pub const $name: Self = Self($val);)*

            pub const fn name(self) -> Option<&'static str> {
                match self.0 {
                    $($(#[$($attrs)*])* $val => Some(std::stringify!($name)),)*
                    _ => None,
                }
            }
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

    MONITOR: 60,
    SETVOLNAME: 61,
    GETXTIMES: 62,
    EXCHANGE: 63,

    // CUSE not currently supported, so this opcode is unused
    #[allow(dead_code)]
    CUSE_INIT: 4096,
}

impl std::fmt::Display for Opcode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(name) = (*self).name() {
            f.write_str(name)
        } else {
            write!(f, "UNKNOWN({})", self.0)
        }
    }
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
                Body::CopyFileRange64(CopyFileRange64::decode_64(buf, ino, cfg)?)
            }

            Opcode::SETVOLNAME => Body::SetVolName(SetVolName::decode(buf, ino, cfg)?),
            Opcode::GETXTIMES => Body::GetXtimes(GetXtimes::decode(buf, ino, cfg)?),
            Opcode::EXCHANGE => Body::Rename(Rename::decode_exchange(buf, ino, cfg)?),
            Opcode::MONITOR => Body::Monitor(Monitor::decode(buf, ino, cfg)?),

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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::buf::BufPool;

    /// Builds a request for `op` on inode 1 with `body` after the header.
    fn request(op: Opcode, body: &[u8]) -> Buf {
        let mut buf = BufPool::new().checkout_with_capacity(HDR_LEN + body.len());

        buf.extend_from_slice(&((HDR_LEN + body.len()) as u32).to_ne_bytes());
        buf.extend_from_slice(&op.0.to_ne_bytes());
        buf.extend_from_slice(&7u64.to_ne_bytes()); // unique
        buf.extend_from_slice(&1u64.to_ne_bytes()); // nodeid
        buf.extend_from_slice(&[0u8; 16]); // uid, gid, pid, padding
        assert_eq!(buf.len(), HDR_LEN);

        buf.extend_from_slice(body);
        buf
    }

    fn decode(op: Opcode, body: &[u8]) -> Body {
        AnyRequest::decode(
            request(op, body),
            crate::handshake::MINOR_VER,
            ReplyInitFlags::empty(),
        )
        .expect("decodes")
        .body
    }

    // No kernel but macOS sends these, but each has one body layout, so a
    // connection speaking for a macFUSE peer is served on any host. Their replies
    // are platform-independent too, so only the inode attributes stay native.
    #[test]
    fn the_macos_only_ops_decode_on_every_platform() {
        assert!(matches!(
            decode(Opcode::SETVOLNAME, b"volume\0"),
            Body::SetVolName(_)
        ));

        assert!(matches!(decode(Opcode::GETXTIMES, &[]), Body::GetXtimes(_)));

        let mut monitor = Vec::new();
        monitor.extend_from_slice(&1u32.to_ne_bytes()); // FUSE_MONITOR_BEGIN
        monitor.extend_from_slice(&0u32.to_ne_bytes()); // padding
        assert!(matches!(
            decode(Opcode::MONITOR, &monitor),
            Body::Monitor(_)
        ));
    }

    // `FUSE_EXCHANGE` arrives as a rename, since that is what it is.
    #[test]
    fn exchange_decodes_on_every_platform() {
        let mut body = Vec::new();
        body.extend_from_slice(&1u64.to_ne_bytes()); // olddir
        body.extend_from_slice(&1u64.to_ne_bytes()); // newdir
        body.extend_from_slice(&0u64.to_ne_bytes()); // options
        body.extend_from_slice(b"old\0new\0");

        let Body::Rename(rename) = decode(Opcode::EXCHANGE, &body) else {
            panic!("expected a rename");
        };

        assert_eq!(rename.rename_mode(), crate::types::RenameMode::ExchangeData);
        assert_eq!(rename.old_name(), "old");
        assert_eq!(rename.new_name(), "new");
    }

    // An opcode no kernel defines has to be refused rather than mistaken for a
    // neighbouring one.
    #[test]
    fn an_unknown_opcode_is_refused() {
        let buf = request(Opcode(9999), &[]);
        let err = AnyRequest::decode(buf, crate::handshake::MINOR_VER, ReplyInitFlags::empty())
            .map(|_| ())
            .unwrap_err();

        assert_eq!(err, Error::ENOSYS);
    }
}
