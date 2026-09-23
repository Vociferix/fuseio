use super::{Cfg, EncodeResp, IntoIoBuf, IoBuf, RawHeader};
use crate::types::{FileFlag, Gid, Ino, InodeKind, SFlag, Uid};

use std::time::{Duration, SystemTime};

#[repr(C)]
#[derive(Debug, Default)]
pub struct Attrs {
    attr_valid: u64,
    attr_valid_nsec: u32,
    _unused: u32,
    attr: InodeAttrs,
}

#[repr(C)]
struct AttrsCompat {
    attr_valid: u64,
    attr_valid_nsec: u32,
    _unused: u32,
    attr: InodeAttrsCompat,
}

#[repr(C)]
#[derive(Debug)]
pub struct InodeAttrs {
    ino: u64,
    size: u64,
    blocks: u64,
    atime: u64,
    mtime: u64,
    ctime: u64,
    #[cfg(target_os = "macos")]
    crtime: u64,
    atimensec: u32,
    mtimensec: u32,
    ctimensec: u32,
    #[cfg(target_os = "macos")]
    crtimensec: u32,
    mode: u32,
    nlink: u32,
    uid: u32,
    gid: u32,
    rdev: u32,
    #[cfg(target_os = "macos")]
    chflags: FileFlag,
    blksize: u32,
    flags: AttrsFlags,
}

const _: () = {
    #[cfg(not(target_os = "macos"))]
    assert!(std::mem::size_of::<InodeAttrs>() == 88);

    #[cfg(target_os = "macos")]
    assert!(std::mem::size_of::<InodeAttrs>() == 104);
};

#[repr(C)]
pub(super) struct InodeAttrsCompat {
    ino: u64,
    size: u64,
    blocks: u64,
    atime: u64,
    mtime: u64,
    ctime: u64,
    atimensec: u32,
    mtimensec: u32,
    ctimensec: u32,
    mode: u32,
}

bitflags::bitflags! {
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    struct AttrsFlags: u32 {
        const SUBMOUNT = 1 << 0;
        const DAX = 1 << 1;
    }
}

impl InodeAttrs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ino(mut self, ino: Ino) -> Self {
        self.ino = ino.as_raw();
        self
    }

    pub fn size(mut self, size: u64) -> Self {
        self.size = size;
        self
    }

    pub fn blocks(mut self, blocks: u64) -> Self {
        self.blocks = blocks;
        self
    }

    pub fn atime(mut self, atime: SystemTime) -> Self {
        let ts = atime
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO);
        self.atime = ts.as_secs();
        self.atimensec = ts.subsec_nanos();
        self
    }

    pub fn mtime(mut self, mtime: SystemTime) -> Self {
        let ts = mtime
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO);
        self.mtime = ts.as_secs();
        self.mtimensec = ts.subsec_nanos();
        self
    }

    pub fn ctime(mut self, ctime: SystemTime) -> Self {
        let ts = ctime
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO);
        self.ctime = ts.as_secs();
        self.ctimensec = ts.subsec_nanos();
        self
    }

    pub fn crtime(mut self, crtime: SystemTime) -> Self {
        #[cfg(target_os = "macos")]
        {
            let ts = crtime
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or(Duration::ZERO);
            self.crtime = ts.as_secs();
            self.crtimensec = ts.subsec_nanos();
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = crtime;
        }

        self
    }

    // TODO: there's no setter for the permission bits or `rdev`, and this
    // overwrites the whole mode, so every attr/entry reply reports mode 0000 and
    // device nodes have no device number.
    // TODO: pre-1970 times are clamped to the epoch here; the wire fields are
    // signed seconds (libfuse passes `st_*time` through), with nsec in [0, 1e9).
    pub fn kind(mut self, kind: InodeKind) -> Self {
        // TODO(e2e): assumes host-native mode values; verify once end-to-end tests
        // can be done.
        self.mode = SFlag::from(kind).bits().into();
        self
    }

    pub fn hard_links(mut self, count: usize) -> Self {
        self.nlink = count.try_into().unwrap_or(u32::MAX);
        self
    }

    pub fn uid(mut self, uid: Uid) -> Self {
        self.uid = uid.as_raw();
        self
    }

    pub fn gid(mut self, gid: Gid) -> Self {
        self.gid = gid.as_raw();
        self
    }

    pub fn block_size(mut self, size: usize) -> Self {
        self.blksize = size.try_into().unwrap_or(u32::MAX);
        self
    }

    pub fn submount_root(mut self, is_submount_root: bool) -> Self {
        self.flags.set(AttrsFlags::SUBMOUNT, is_submount_root);
        self
    }

    pub fn flags(mut self, flags: FileFlag) -> Self {
        #[cfg(target_os = "macos")]
        {
            self.chflags = flags;
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = flags;
        }

        self
    }

    pub fn add_flags(mut self, flags: FileFlag) -> Self {
        #[cfg(target_os = "macos")]
        {
            self.chflags.insert(flags);
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = flags;
        }

        self
    }

    pub fn clear_flags(mut self, flags: FileFlag) -> Self {
        #[cfg(target_os = "macos")]
        {
            self.chflags.remove(flags);
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = flags;
        }

        self
    }
}

impl Attrs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.attr_valid = ttl.as_secs();
        self.attr_valid_nsec = ttl.subsec_nanos();
        self
    }

    pub fn attrs(mut self, attrs: InodeAttrs) -> Self {
        self.attr = attrs;
        self
    }
}

impl EncodeResp for Attrs {
    type Error = std::convert::Infallible;

    fn encode(self, id: u64, cfg: Cfg) -> Result<impl IntoIoBuf, Self::Error> {
        #[repr(C)]
        struct Out {
            hdr: RawHeader,
            attr: Attrs,
        }

        #[repr(C)]
        struct OutCompat {
            hdr: RawHeader,
            attr: AttrsCompat,
        }

        impl IoBuf for Out {
            fn as_init(&self) -> &[u8] {
                unsafe { std::slice::from_raw_parts(self.buf_ptr(), self.buf_len()) }
            }

            fn buf_ptr(&self) -> *const u8 {
                self as *const Self as *const u8
            }

            fn buf_len(&self) -> usize {
                self.hdr.len as usize
            }
        }

        let len = if !cfg!(target_os = "macos") && cfg.minor_ver < 9 {
            std::mem::size_of::<OutCompat>()
        } else {
            std::mem::size_of::<Out>()
        };

        Ok(Out {
            hdr: RawHeader {
                len: len as u32,
                err: 0,
                id,
            },
            attr: self,
        })
    }
}

impl Default for InodeAttrs {
    fn default() -> Self {
        #[allow(unused_mut)]
        let mut attrs: Self = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };

        #[cfg(target_os = "macos")]
        {
            attrs.crtime = u64::MAX;
            attrs.crtimensec = u32::MAX;
        }

        attrs
    }
}
