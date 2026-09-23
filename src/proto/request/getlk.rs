use super::{Cfg, HDR_LEN, Ino};
use crate::types::{FileHandle, FileRange, LockFlags, LockKind, LockOwner, Pid};
use crate::{Error, Result, buf::Buf};

#[derive(Debug)]
pub struct GetLk {
    ino: Ino,
    fh: FileHandle,
    owner: Option<LockOwner>,
    range: FileRange,
    kind: LockKind,
    pid: Option<Pid>,
    is_flock: bool,
}

#[repr(C)]
pub(super) struct Raw {
    pub(super) fh: u64,
    pub(super) owner: u64,
    pub(super) start: u64,
    pub(super) end: u64,
    pub(super) kind: u32,
    pub(super) pid: u32,
    pub(super) lk_flags: LockFlags,
    _unused: u32,
}

impl GetLk {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> FileHandle {
        self.fh
    }

    pub fn lock_owner(&self) -> Option<LockOwner> {
        self.owner
    }

    pub fn file_range(&self) -> FileRange {
        self.range.clone()
    }

    pub fn lock_kind(&self) -> LockKind {
        self.kind
    }

    pub fn pid(&self) -> Option<Pid> {
        self.pid
    }

    pub fn is_flock(&self) -> bool {
        self.is_flock
    }
}

impl GetLk {
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, _: Cfg) -> Result<Self> {
        if buf.len() < const { HDR_LEN + std::mem::size_of::<Raw>() } {
            return Err(Error::EPROTO);
        }

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        let raw = unsafe { &*(buf.as_ptr().add(HDR_LEN) as *const Raw) };

        // TODO(e2e): assumes host-native F_*LCK values; verify once end-to-end
        // tests can be done.
        // `F_*LCK` are `c_short` on macOS and FreeBSD.
        const RDLCK: i32 = nix::libc::F_RDLCK as i32;
        const WRLCK: i32 = nix::libc::F_WRLCK as i32;
        const UNLCK: i32 = nix::libc::F_UNLCK as i32;

        let kind = match raw.kind.cast_signed() {
            RDLCK => LockKind::Read,
            WRLCK => LockKind::Write,
            UNLCK => LockKind::Unlock,
            _ => return Err(Error::EINVAL),
        };

        // TODO: libfuse passes `owner` to flock handlers too (it's what a later
        // RELEASE with FLOCK_UNLOCK carries); it's dropped here and `FlockReq`
        // has no `lock_owner()`.
        Ok(if raw.lk_flags.contains(LockFlags::FLOCK) {
            Self {
                ino,
                fh: FileHandle(raw.fh),
                owner: None,
                range: FileRange::Open(0..),
                kind,
                pid: None,
                is_flock: true,
            }
        } else {
            Self {
                ino,
                fh: FileHandle(raw.fh),
                owner: LockOwner::try_from(raw.owner).ok(),
                range: FileRange::new(raw.start, raw.end),
                kind,
                pid: Some(Pid::from_raw(raw.pid.cast_signed())),
                is_flock: false,
            }
        })
    }
}
