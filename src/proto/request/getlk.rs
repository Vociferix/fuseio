use super::{Cfg, Ino, RawHeader};
use crate::types::{FileHandle, FileRange, LockFlags, LockKind, LockOwner, Tgid};
use crate::{Error, Result, buf::Buf};

#[derive(Debug)]
pub struct GetLk {
    ino: Ino,
    fh: FileHandle,
    owner: Option<LockOwner>,
    range: FileRange,
    kind: LockKind,
    tgid: Option<Tgid>,
    is_flock: bool,
}

#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
pub(super) struct Raw {
    _hdr: RawHeader,
    pub(super) fh: u64,
    pub(super) owner: u64,
    pub(super) start: u64,
    pub(super) end: u64,
    pub(super) kind: u32,
    pub(super) pid: u32,
    pub(super) lk_flags: LockFlags,
    _unused: u32,
}

/// The body before 7.9, which has no lock flags and so is never a `flock`.
#[derive(Clone, Copy, bytemuck::AnyBitPattern)]
#[repr(C)]
struct RawCompat {
    hdr: RawHeader,
    fh: u64,
    owner: u64,
    start: u64,
    end: u64,
    kind: u32,
    pid: u32,
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
        self.range
    }

    pub fn lock_kind(&self) -> LockKind {
        self.kind
    }

    /// The thread group that asked for the lock, if any: a kernel sends none
    /// when releasing one.
    pub fn tgid(&self) -> Option<Tgid> {
        self.tgid
    }

    pub fn is_flock(&self) -> bool {
        self.is_flock
    }
}

#[allow(clippy::unnecessary_cast)]
fn lock_kind(kind: u32) -> Result<LockKind> {
    // TODO(e2e): assumes host-native F_*LCK values; verify once end-to-end
    // tests can be done.
    // `F_*LCK` are `c_short` on macOS and FreeBSD.
    const RDLCK: i32 = nix::libc::F_RDLCK as i32;
    const WRLCK: i32 = nix::libc::F_WRLCK as i32;
    const UNLCK: i32 = nix::libc::F_UNLCK as i32;

    match kind.cast_signed() {
        RDLCK => Ok(LockKind::Read),
        WRLCK => Ok(LockKind::Write),
        UNLCK => Ok(LockKind::Unlock),
        _ => Err(Error::EINVAL),
    }
}

impl GetLk {
    // `F_*LCK` are `c_int` on Linux and `c_short` elsewhere.
    #[allow(clippy::unnecessary_cast)]
    pub(super) fn decode(buf: Buf, ino: Option<Ino>, cfg: Cfg) -> Result<Self> {
        if cfg.minor_ver < 9 {
            let Some((raw, _)) = buf.as_slice().split_first::<RawCompat>() else {
                return Err(Error::EPROTO);
            };

            let Some(ino) = ino else {
                return Err(Error::EINVAL);
            };

            return Ok(Self {
                ino,
                fh: FileHandle(raw.fh),
                owner: LockOwner::try_from(raw.owner).ok(),
                range: FileRange::new(raw.start, raw.end),
                kind: lock_kind(raw.kind)?,
                tgid: Tgid::from_raw(raw.pid.cast_signed()),
                is_flock: false,
            });
        }

        let Some((raw, _)) = buf.as_slice().split_first::<Raw>() else {
            return Err(Error::EPROTO);
        };

        let Some(ino) = ino else {
            return Err(Error::EINVAL);
        };

        Ok(if raw.lk_flags.contains(LockFlags::FLOCK) {
            Self {
                ino,
                fh: FileHandle(raw.fh),
                owner: LockOwner::try_from(raw.owner).ok(),
                range: FileRange::Open { offset: 0 },
                kind: lock_kind(raw.kind)?,
                tgid: Tgid::from_raw(raw.pid.cast_signed()),
                is_flock: true,
            }
        } else {
            Self {
                ino,
                fh: FileHandle(raw.fh),
                owner: LockOwner::try_from(raw.owner).ok(),
                range: FileRange::new(raw.start, raw.end),
                kind: lock_kind(raw.kind)?,
                tgid: Tgid::from_raw(raw.pid.cast_signed()),
                is_flock: false,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::buf::BufPool;

    const FLOCK: u32 = 1 << 0;

    fn cfg(minor_ver: u32) -> Cfg {
        Cfg {
            minor_ver,
            ..Cfg::default()
        }
    }

    #[allow(clippy::unnecessary_cast)]
    fn request(lk_flags: Option<u32>) -> Buf {
        let mut buf = BufPool::new().checkout_with_capacity(64);

        buf.extend_from_slice(&[0u8; std::mem::size_of::<RawHeader>()]);
        buf.extend_from_slice(&9u64.to_ne_bytes()); // fh
        buf.extend_from_slice(&42u64.to_ne_bytes()); // owner
        buf.extend_from_slice(&1024u64.to_ne_bytes()); // start
        buf.extend_from_slice(&2047u64.to_ne_bytes()); // end
        buf.extend_from_slice(&(nix::libc::F_WRLCK as i32).to_ne_bytes());
        buf.extend_from_slice(&77u32.to_ne_bytes()); // pid

        if let Some(lk_flags) = lk_flags {
            buf.extend_from_slice(&lk_flags.to_ne_bytes());
            buf.extend_from_slice(&0u32.to_ne_bytes()); // padding
        }

        buf
    }

    #[test]
    fn a_posix_lock_decodes() {
        let req = GetLk::decode(request(Some(0)), Ino::from_raw(1), cfg(45)).unwrap();

        assert_eq!(req.file_handle(), FileHandle(9));
        assert_eq!(req.lock_owner().map(u64::from), Some(42));
        assert_eq!(req.lock_kind(), LockKind::Write);
        assert_eq!(req.tgid().map(Tgid::as_raw), Some(77));
        assert!(!req.is_flock());
        assert_eq!(req.file_range().start_offset(), 1024);
        assert_eq!(req.file_range().end_offset(), Some(2047));
    }

    // A flock's owner names the open file, which is what a later RELEASE
    // carries, so it has to survive decoding.
    #[test]
    fn an_flock_decodes() {
        let req = GetLk::decode(request(Some(FLOCK)), Ino::from_raw(1), cfg(45)).unwrap();

        assert!(req.is_flock());
        assert_eq!(req.lock_kind(), LockKind::Write);
        assert_eq!(req.lock_owner().map(u64::from), Some(42));
        assert_eq!(req.tgid().map(Tgid::as_raw), Some(77));
    }

    #[test]
    fn a_release_carries_no_thread_group() {
        let mut buf = request(Some(FLOCK));
        let pid = std::mem::size_of::<RawHeader>() + 8 + 8 + 8 + 8 + 4;
        buf[pid..pid + 4].copy_from_slice(&0u32.to_ne_bytes());

        let req = GetLk::decode(buf, Ino::from_raw(1), cfg(45)).unwrap();

        assert!(req.tgid().is_none());
    }

    // Before 7.9 the body has no lock flags, so no lock is ever a flock.
    #[test]
    fn an_old_kernels_body_decodes() {
        let req = GetLk::decode(request(None), Ino::from_raw(1), cfg(8)).unwrap();

        assert!(!req.is_flock());
        assert_eq!(req.file_handle(), FileHandle(9));
        assert_eq!(req.lock_owner().map(u64::from), Some(42));
        assert_eq!(req.file_range().start_offset(), 1024);
    }

    #[test]
    fn a_short_body_is_rejected() {
        assert_eq!(
            GetLk::decode(request(None), Ino::from_raw(1), cfg(45)).unwrap_err(),
            Error::EPROTO
        );
    }

    #[test]
    fn an_unknown_lock_kind_is_rejected() {
        let mut buf = request(Some(0));
        let kind = std::mem::size_of::<RawHeader>() + 8 + 8 + 8 + 8;
        buf[kind..kind + 4].copy_from_slice(&99u32.to_ne_bytes());

        assert_eq!(
            GetLk::decode(buf, Ino::from_raw(1), cfg(45)).unwrap_err(),
            Error::EINVAL
        );
    }
}
