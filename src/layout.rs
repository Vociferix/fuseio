use crate::{Error, Result};
use bytemuck::{Pod, Zeroable};

pub const VERSION_MAJOR: u32 = 7;

#[cfg(target_os = "macos")]
pub const VERSION_MINOR: u32 = 19;

#[cfg(not(target_os = "macos"))]
pub const VERSION_MINOR: u32 = 45;

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable)]
pub struct MsgOut<T> {
    pub hdr: HeaderOut,
    pub body: T,
}

impl<T: Pod> MsgOut<T> {
    pub fn new(unique: u64, body: T) -> Self {
        Self {
            hdr: HeaderOut {
                len: (std::mem::size_of::<HeaderOut>() + std::mem::size_of::<T>()) as u32,
                error: 0,
                unique,
            },
            body,
        }
    }
}

unsafe impl<T: Pod> Pod for MsgOut<T> {}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct Attr {
    pub ino: u64,
    pub size: u64,
    pub block: u64,
    pub atime: u64,
    pub mtime: u64,
    pub ctime: u64,
    #[cfg(target_os = "macos")]
    pub crtime: u64,
    pub atimensec: u32,
    pub mtimensec: u32,
    pub ctimensec: u32,
    #[cfg(target_os = "macos")]
    pub crtimensec: u32,
    pub mode: u32,
    pub nlink: u32,
    pub uid: u32,
    pub gid: u32,
    pub rdev: u32,
    #[cfg(target_os = "macos")]
    pub flags: u32,
    pub blksize: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct AttrCompat {
    pub ino: u64,
    pub size: u64,
    pub block: u64,
    pub atime: u64,
    pub mtime: u64,
    pub ctime: u64,
    pub atimensec: u32,
    pub mtimensec: u32,
    pub ctimensec: u32,
    pub mode: u32,
}

impl Attr {
    pub fn compat(self) -> AttrCompat {
        unsafe { std::ptr::read(&raw const self as *const AttrCompat) }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct KStatFs {
    pub blocks: u64,
    pub bfree: u64,
    pub bavail: u64,
    pub files: u64,
    pub ffree: u64,
    pub bsize: u32,
    pub namelen: u32,
    pub frsize: u32,
    pub padding: u32,
    pub spare: [u32; 6],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct KStatFsCompat {
    pub blocks: u64,
    pub bfree: u64,
    pub bavail: u64,
    pub files: u64,
    pub ffree: u64,
    pub bsize: u32,
    pub namelen: u32,
}

impl KStatFs {
    pub fn compat(self) -> KStatFsCompat {
        unsafe { std::ptr::read(&raw const self as *const KStatFsCompat) }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct FileLock {
    pub start: u64,
    pub end: u64,
    pub typ: LockOp,
    pub pid: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct LockOp(pub u32);

impl LockOp {
    pub const READ: Self = Self(nix::libc::F_RDLCK.cast_unsigned());
    pub const WRITE: Self = Self(nix::libc::F_WRLCK.cast_unsigned());
    pub const UNLOCK: Self = Self(nix::libc::F_UNLCK.cast_unsigned());
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct Opcode(pub u32);

macro_rules! opcodes {
    ($($(#[$($attrs:tt)*])* $name:ident : $val:literal),* $(,)?) => {
        impl Opcode {
            $($(#[$($attrs)*])* pub const $name: Self = Self($val);)*
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

    #[cfg(target_os = "macos")]
    SETVOLNAME: 61,
    #[cfg(target_os = "macos")]
    GETXTIMES: 62,
    #[cfg(target_os = "macos")]
    EXCHANGE: 63,

    CUSE_INIT: 4096,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct NotifyCode(pub u32);

impl NotifyCode {
    pub const POLL: Self = Self(1);
    pub const NOTIFY_INVAL_INODE: Self = Self(2);
    pub const NOTIFY_INVAL_ENTRY: Self = Self(3);
    pub const NOTIFY_STORE: Self = Self(4);
    pub const NOTIFY_RETRIEVE: Self = Self(5);
    pub const NOTIFY_DELETE: Self = Self(6);
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct EntryOut {
    pub nodeid: u64,
    pub generation: u64,
    pub entry_valid: u64,
    pub attr_valid: u64,
    pub entry_valid_nsec: u32,
    pub attr_valid_nsec: u32,
    pub attr: Attr,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct EntryOutCompat {
    pub nodeid: u64,
    pub generation: u64,
    pub entry_valid: u64,
    pub attr_valid: u64,
    pub entry_valid_nsec: u32,
    pub attr_valid_nsec: u32,
    pub attr: AttrCompat,
}

impl EntryOut {
    pub fn compat(self) -> EntryOutCompat {
        unsafe { std::ptr::read(&raw const self as *const EntryOutCompat) }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct ForgetIn {
    pub nlookup: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct ForgetOne {
    pub nodeid: u64,
    pub nlookup: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct BatchForgetIn {
    pub count: u32,
    pub dummy: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct GetAttrIn {
    pub getattr_flags: GetAttrFlags,
    pub dummy: u32,
    pub fh: u64,
}

bitflags::bitflags! {
    #[repr(C)]
    #[derive(Debug, Clone, Copy, Zeroable, Pod)]
    pub struct GetAttrFlags: u32 {
        const HAVE_FH = 1;
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct AttrOut {
    pub attr_valid: u64,
    pub attr_valid_nsec: u32,
    pub dummy: u32,
    pub attr: Attr,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct AttrOutCompat {
    pub attr_valid: u64,
    pub attr_valid_nsec: u32,
    pub dummy: u32,
    pub attr: AttrCompat,
}

impl AttrOut {
    pub fn compat(self) -> AttrOutCompat {
        unsafe { std::ptr::read(&raw const self as *const AttrOutCompat) }
    }
}

#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct GetXtimesOut {
    pub bkuptime: u64,
    pub crtime: u64,
    pub bkuptimensec: u32,
    pub crtimensec: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct MknodIn {
    pub mode: u32,
    pub rdev: u32,
    pub umask: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct MknodInCompat {
    pub mode: u32,
    pub rdev: u32,
}

impl MknodIn {
    pub fn from_compat(compat: MknodInCompat) -> Self {
        Self {
            mode: compat.mode,
            rdev: compat.rdev,
            umask: const { nix::sys::stat::Mode::all().bits() },
            padding: 0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct MkdirIn {
    pub mode: u32,
    pub umask: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct RenameIn {
    pub newdir: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct Rename2In {
    pub newdir: u64,
    pub flags: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct ExchangeIn {
    pub olddir: u64,
    pub newdir: u64,
    pub options: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct LinkIn {
    pub oldnodeid: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct SetAttrIn {
    pub valid: u32,
    pub padding: u32,
    pub fh: u64,
    pub size: u64,
    pub lock_owner: u64,
    pub atime: u64,
    pub mtime: u64,
    pub ctime: u64,
    pub atimensec: u32,
    pub mtimensec: u32,
    pub ctimensec: u32,
    pub mode: u32,
    pub _unused0: u32,
    pub uid: u32,
    pub gid: u32,
    pub _unused1: u32,
    #[cfg(target_os = "macos")]
    pub bkuptime: u64,
    #[cfg(target_os = "macos")]
    pub chgtime: u64,
    #[cfg(target_os = "macos")]
    pub crtime: u64,
    #[cfg(target_os = "macos")]
    pub bkuptimensec: u32,
    #[cfg(target_os = "macos")]
    pub chgtimensec: u32,
    #[cfg(target_os = "macos")]
    pub crtimensec: u32,
    #[cfg(target_os = "macos")]
    pub flags: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct OpenIn {
    pub flags: u32,
    pub _unused: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct CreateIn {
    pub flags: u32,
    pub mode: u32,
    pub umask: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct CreateOut {
    pub entry: EntryOut,
    pub open: OpenOut,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct CreateOutCompat {
    pub entry: EntryOutCompat,
    pub open: OpenOut,
}

impl CreateOut {
    pub fn compat(self) -> CreateOutCompat {
        unsafe { std::ptr::read(&raw const self as *const CreateOutCompat) }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct OpenOut {
    pub fh: u64,
    pub open_flags: OpenFlags,
    pub backing_id: u32,
}

bitflags::bitflags! {
    #[repr(C)]
    #[derive(Debug, Clone, Copy, Zeroable, Pod)]
    pub struct OpenFlags: u32 {
        const DIRECT_IO = 1 << 0;
        const KEEP_CACHE = 1 << 1;
        const NONSEEKABLE = 1 << 2;
        const CACHE_DIR = 1 << 3;
        const STREAM = 1 << 4;
        const NO_FLUSH = 1 << 5;
        const PARALLEL_DIRECT_WRITES = 1 << 6;
        const PASSTHROUGH = 1 << 7;
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct ReleaseIn {
    pub fh: u64,
    pub flags: u32,
    pub release_flags: u32,
    pub lock_owner: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct FlushIn {
    pub fh: u64,
    pub _unused: u32,
    pub padding: u32,
    pub lock_owner: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct ReadIn {
    pub fh: u64,
    pub offset: u64,
    pub size: u32,
    pub read_flags: u32,
    pub lock_owner: u64,
    pub flags: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct WriteIn {
    pub fh: u64,
    pub offset: u64,
    pub size: u32,
    pub write_flags: u32,
    pub lock_owner: u64,
    pub flags: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct WriteInCompat {
    pub fh: u64,
    pub offset: u64,
    pub size: u32,
    pub write_flags: u32,
}

impl WriteIn {
    pub fn from_compat(compat: WriteInCompat) -> Self {
        Self {
            fh: compat.fh,
            offset: compat.offset,
            size: compat.size,
            write_flags: compat.write_flags,
            lock_owner: 0,
            flags: 0,
            padding: 0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct WriteOut {
    pub size: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct StatFsOut {
    pub st: KStatFs,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct StatFsOutCompat {
    pub st: KStatFsCompat,
}

impl StatFsOut {
    pub fn compat(self) -> StatFsOutCompat {
        unsafe { std::ptr::read(&raw const self as *const StatFsOutCompat) }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct FsyncIn {
    pub fh: u64,
    pub fsync_flags: FsyncFlags,
    pub padding: u32,
}

bitflags::bitflags! {
    #[repr(C)]
    #[derive(Debug, Clone, Copy, Zeroable, Pod)]
    pub struct FsyncFlags: u32 {
        const DATASYNC = 1;
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct SetXattrIn {
    pub size: u32,
    pub flags: u32,
    #[cfg(not(target_os = "macos"))]
    pub setxattr_flags: u32,
    #[cfg(target_os = "macos")]
    pub position: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct SetXattrInCompat {
    pub size: u32,
    pub flags: u32,
}

impl SetXattrIn {
    pub fn from_compat(compat: SetXattrInCompat) -> Self {
        Self {
            size: compat.size,
            flags: compat.flags,
            #[cfg(not(target_os = "macos"))]
            setxattr_flags: 0,
            #[cfg(target_os = "macos")]
            position: 0,
            padding: 0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct GetXattrIn {
    pub size: u32,
    pub padding: u32,
    #[cfg(target_os = "macos")]
    pub position: u32,
    #[cfg(target_os = "macos")]
    pub padding2: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct GetXattrOut {
    pub size: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct LockIn {
    pub fh: u64,
    pub owner: u64,
    pub lk: FileLock,
    pub lk_flags: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct LockOut {
    pub lk: FileLock,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct AccessIn {
    pub mask: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct InitIn {
    pub major: u32,
    pub minor: u32,
    pub max_readahead: u32,
    pub flags: u32,
    pub flags2: u32,
    pub _unused: [u32; 11],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct InitOut {
    pub major: u32,
    pub minor: u32,
    pub max_readahead: u32,
    pub flags: u32,
    pub max_background: u16,
    pub congestion_threshold: u16,
    pub max_write: u32,
    pub time_gran: u32,
    pub max_pages: u16,
    pub _unused: u16,
    pub flags2: u32,
    pub max_stack_depth: u32,
    pub _reserved: [u32; 6],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct InitOutCompat {
    pub major: u32,
    pub minor: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct InitOutCompat22 {
    pub major: u32,
    pub minor: u32,
    pub max_readahead: u32,
    pub flags: u32,
    pub max_background: u16,
    pub congestion_threshold: u16,
    pub max_write: u32,
}

impl InitOut {
    pub fn compat(self) -> InitOutCompat {
        unsafe { std::ptr::read(&raw const self as *const InitOutCompat) }
    }

    pub fn compat22(self) -> InitOutCompat22 {
        unsafe { std::ptr::read(&raw const self as *const InitOutCompat22) }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct CuseInitIn {
    pub major: u32,
    pub minor: u32,
    pub _unused: u32,
    pub flags: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct CuseInitOut {
    pub major: u32,
    pub minor: u32,
    pub _unused: u32,
    pub flags: u32,
    pub max_read: u32,
    pub max_write: u32,
    pub dev_major: u32,
    pub dev_minor: u32,
    pub _spare: [u32; 10],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct InterruptIn {
    pub unique: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct BmapIn {
    pub block: u64,
    pub blocksize: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct IoctlIn {
    pub fh: u64,
    pub flags: u32,
    pub cmd: u32,
    pub arg: u64,
    pub in_size: u32,
    pub out_size: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct IoctlIovec {
    pub base: u64,
    pub len: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct IoctlOut {
    pub result: i32,
    pub flags: u32,
    pub in_iovs: u32,
    pub out_iovs: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct PollIn {
    pub fh: u64,
    pub kh: u64,
    pub flags: u32,
    pub events: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct PollOut {
    pub revents: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct NotifyPollWakeupOut {
    pub kh: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct FallocateIn {
    pub fh: u64,
    pub offset: u64,
    pub length: u64,
    pub mode: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct HeaderIn {
    pub len: u32,
    pub opcode: Opcode,
    pub unique: u64,
    pub nodeid: u64,
    pub uid: u32,
    pub gid: u32,
    pub pid: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct HeaderOut {
    pub len: u32,
    pub error: i32,
    pub unique: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct Dirent {
    pub ino: u64,
    pub off: u64,
    pub namelen: u32,
    pub typ: u32,
    //pub name: [u8; namelen]
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct DirentPlus {
    pub entry_out: EntryOut,
    pub direct: Dirent,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct NotifyInvalInodeOut {
    pub ino: u64,
    pub off: i64,
    pub len: i64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct NotifyInvalEntryOut {
    pub parent: u64,
    pub namelen: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct NotifyDeleteOut {
    pub parent: u64,
    pub child: u64,
    pub namelen: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct NotifyStoreOut {
    pub nodeid: u64,
    pub offset: u64,
    pub size: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct NotifyRetrieveOut {
    pub notify_unique: u64,
    pub nodeid: u64,
    pub offset: u64,
    pub size: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct NotifyRetrieveIn {
    pub _dummy0: u64,
    pub offset: u64,
    pub size: u32,
    pub _dummy1: u32,
    pub _dummy3: u64,
    pub _dummy4: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct LseekIn {
    pub fh: u64,
    pub offset: i64,
    pub whence: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct LseekOut {
    pub offset: i64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct CopyFileRangeIn {
    pub fh_in: u64,
    pub off_in: u64,
    pub nodeid_out: u64,
    pub fh_out: u64,
    pub off_out: u64,
    pub len: u64,
    pub flags: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct BackingMap {
    pub fd: i32,
    pub flags: u32,
    pub padding: u64,
}

nix::ioctl_write_ptr!(passthrough_open, 299, 1, BackingMap);
nix::ioctl_write_ptr!(passthrough_close, 299, 2, u32);
