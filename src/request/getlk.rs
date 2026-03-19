use super::{Body, FileHandle, Ino, LockOwner, Pid, Request, decode, handle_error, send_error};
use crate::async_rc::AsyncRc;
use crate::layout::{FileLock as RawFileLock, LockIn, LockOp, MsgOut};
use crate::serve::Server;
use crate::{Error, Filesystem, Result};

use compio::runtime::spawn;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LockKind {
    Unlock,
    Read,
    Write,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FileRange {
    Closed(std::ops::RangeInclusive<u64>),
    Open(std::ops::RangeFrom<u64>),
}

#[derive(Debug)]
pub struct GetLockReq {
    req: Request,
    ino: Ino,
    fh: FileHandle,
    owner: LockOwner,
    pid: Pid,
    range: FileRange,
    kind: LockKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileLock {
    pub(super) kind: LockKind,
    pub(super) pid: Pid,
    pub(super) range: FileRange,
}

const OFFSET_MAX: u64 = 0x7fffffffffffffff;

impl FileRange {
    pub(super) fn new(start: u64, end: u64) -> Self {
        if end == OFFSET_MAX {
            Self::Open(start..)
        } else {
            Self::Closed(start..=end)
        }
    }

    pub fn start_offset(&self) -> u64 {
        match self {
            Self::Closed(range) => *range.start(),
            Self::Open(range) => range.start,
        }
    }

    pub fn end_offset(&self) -> Option<u64> {
        if let Self::Closed(range) = self {
            Some(*range.end())
        } else {
            None
        }
    }

    pub fn len(&self) -> Option<u64> {
        if let Self::Closed(range) = self {
            Some(*range.end() - *range.start() + 1)
        } else {
            None
        }
    }
}

impl From<std::ops::Range<u64>> for FileRange {
    fn from(range: std::ops::Range<u64>) -> Self {
        Self::Closed(range.start..=range.end.saturating_sub(1))
    }
}

impl From<std::ops::RangeFrom<u64>> for FileRange {
    fn from(range: std::ops::RangeFrom<u64>) -> Self {
        Self::Open(range)
    }
}

impl From<std::ops::RangeInclusive<u64>> for FileRange {
    fn from(range: std::ops::RangeInclusive<u64>) -> Self {
        Self::Closed(range)
    }
}

impl From<std::ops::RangeTo<u64>> for FileRange {
    fn from(range: std::ops::RangeTo<u64>) -> Self {
        Self::Closed(0..=range.end.saturating_sub(1))
    }
}

impl From<std::ops::RangeToInclusive<u64>> for FileRange {
    fn from(range: std::ops::RangeToInclusive<u64>) -> Self {
        Self::Closed(0..=range.end)
    }
}

impl std::ops::RangeBounds<u64> for FileRange {
    fn start_bound(&self) -> std::ops::Bound<&u64> {
        match self {
            Self::Closed(range) => std::ops::Bound::Included(range.start()),
            Self::Open(range) => std::ops::Bound::Included(&range.start),
        }
    }

    fn end_bound(&self) -> std::ops::Bound<&u64> {
        if let Self::Closed(range) = self {
            std::ops::Bound::Included(range.end())
        } else {
            std::ops::Bound::Unbounded
        }
    }
}

impl GetLockReq {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> FileHandle {
        self.fh
    }

    pub fn lock_owner(&self) -> LockOwner {
        self.owner
    }

    pub fn pid(&self) -> Pid {
        self.pid
    }

    pub fn file_range(&self) -> FileRange {
        self.range.clone()
    }

    pub fn lock_kind(&self) -> LockKind {
        self.kind
    }
}

impl std::ops::Deref for GetLockReq {
    type Target = Request;

    fn deref(&self) -> &Request {
        &self.req
    }
}

impl FileLock {
    pub fn new() -> Self {
        Self {
            kind: LockKind::Unlock,
            pid: Pid::from_raw(-1),
            range: FileRange::Open(0..),
        }
    }

    pub fn kind(mut self, kind: LockKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn pid(mut self, pid: Pid) -> Self {
        self.pid = pid;
        self
    }

    pub fn range(mut self, range: impl Into<FileRange>) -> Self {
        self.range = range.into();
        self
    }

    pub(super) fn build(self) -> RawFileLock {
        let Self { kind, pid, range } = self;

        let typ = match kind {
            LockKind::Read => LockOp::READ,
            LockKind::Write => LockOp::WRITE,
            LockKind::Unlock => LockOp::UNLOCK,
        };

        RawFileLock {
            start: range.start_offset(),
            end: range.end_offset().unwrap_or(OFFSET_MAX),
            typ,
            pid: pid.as_raw().cast_unsigned(),
        }
    }
}

impl Server {
    pub fn getlk<F>(&self, fs: &AsyncRc<F>, req: Request, ino: Ino, body: Body) -> Result<()>
    where
        F: Filesystem,
    {
        let body = decode::<LockIn>(body)?.0;

        if body.lk.start > body.lk.end {
            return Err(Error::EINVAL);
        }

        let kind = match body.lk.typ {
            LockOp::READ => LockKind::Read,
            LockOp::WRITE => LockKind::Write,
            LockOp::UNLOCK => LockKind::Unlock,
            _ => return Err(Error::EINVAL),
        };

        let fs = fs.clone();
        let mut tx = self.tx.clone();

        spawn(async move {
            let ureq = GetLockReq {
                req,
                ino,
                fh: FileHandle(body.fh),
                owner: LockOwner(body.owner),
                pid: Pid::from_raw(body.lk.pid.cast_signed()),
                range: FileRange::new(body.lk.start, body.lk.end),
                kind,
            };

            handle_error(match fs.get_lock(ureq).await {
                Ok(lock) => tx.send(MsgOut::new(req.id(), lock.build())).await,
                Err(err) => send_error(err, req.id(), &mut tx).await,
            });
        })
        .detach();

        Ok(())
    }
}
