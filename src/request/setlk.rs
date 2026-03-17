use super::{
    Body, FileHandle, FileRange, FlockArg, Ino, LockKind, LockOwner, Pid, Request, decode,
    handle_error, send_result,
};
use crate::async_rc::AsyncRc;
use crate::layout::{LockFlags, LockIn, LockOp};
use crate::serve::Server;
use crate::{Error, Filesystem, Result};

use compio::runtime::spawn;

#[derive(Debug)]
pub struct SetLockReq {
    req: Request,
    ino: Ino,
    fh: FileHandle,
    owner: LockOwner,
    pid: Pid,
    range: FileRange,
    kind: LockKind,
    block: bool,
}

#[derive(Debug)]
pub struct FlockReq {
    req: Request,
    ino: Ino,
    fh: FileHandle,
    owner: LockOwner,
    op: FlockArg,
}

impl SetLockReq {
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

    pub fn is_blocking(&self) -> bool {
        self.block
    }
}

impl std::ops::Deref for SetLockReq {
    type Target = Request;

    fn deref(&self) -> &Request {
        &self.req
    }
}

impl FlockReq {
    pub fn ino(&self) -> Ino {
        self.ino
    }

    pub fn file_handle(&self) -> FileHandle {
        self.fh
    }

    pub fn lock_owner(&self) -> LockOwner {
        self.owner
    }

    pub fn op(&self) -> FlockArg {
        self.op
    }

    pub fn file_range(&self) -> FileRange {
        FileRange::Open(0..)
    }

    pub fn lock_kind(&self) -> LockKind {
        match self.op {
            FlockArg::Unlock | FlockArg::UnlockNonblock => LockKind::Unlock,
            FlockArg::LockShared | FlockArg::LockSharedNonblock => LockKind::Read,
            FlockArg::LockExclusive | FlockArg::LockExclusiveNonblock => LockKind::Write,
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    pub fn is_blocking(&self) -> bool {
        matches!(
            self.op,
            FlockArg::Unlock | FlockArg::LockShared | FlockArg::LockExclusive
        )
    }
}

impl std::ops::Deref for FlockReq {
    type Target = Request;

    fn deref(&self) -> &Request {
        &self.req
    }
}

impl Server {
    pub fn setlkw<F>(&self, fs: &AsyncRc<F>, req: Request, ino: Ino, body: Body) -> Result<()>
    where
        F: Filesystem,
    {
        self.setlock(fs, req, ino, body, true)
    }

    pub fn setlk<F>(&self, fs: &AsyncRc<F>, req: Request, ino: Ino, body: Body) -> Result<()>
    where
        F: Filesystem,
    {
        self.setlock(fs, req, ino, body, false)
    }

    fn setlock<F>(
        &self,
        fs: &AsyncRc<F>,
        req: Request,
        ino: Ino,
        body: Body,
        block: bool,
    ) -> Result<()>
    where
        F: Filesystem,
    {
        let body = decode::<LockIn>(body)?.0;

        if body.lk_flags.contains(LockFlags::FLOCK) {
            self.set_bsd_lock(fs, req, ino, body, block)
        } else {
            self.set_posix_lock(fs, req, ino, body, block)
        }
    }

    fn set_bsd_lock<F>(
        &self,
        fs: &AsyncRc<F>,
        req: Request,
        ino: Ino,
        body: LockIn,
        block: bool,
    ) -> Result<()>
    where
        F: Filesystem,
    {
        let op = match (body.lk.typ, block) {
            (LockOp::UNLOCK, false) => FlockArg::UnlockNonblock,
            (LockOp::UNLOCK, true) => FlockArg::Unlock,
            (LockOp::READ, false) => FlockArg::LockSharedNonblock,
            (LockOp::READ, true) => FlockArg::LockShared,
            (LockOp::WRITE, false) => FlockArg::LockExclusiveNonblock,
            (LockOp::WRITE, true) => FlockArg::LockExclusive,
            _ => return Err(Error::EINVAL),
        };

        let fs = fs.clone();
        let mut tx = self.tx.clone();

        spawn(async move {
            let req = FlockReq {
                req,
                ino,
                fh: FileHandle(body.fh),
                owner: LockOwner(body.owner),
                op,
            };
            handle_error(send_result(fs.flock(&req).await, req.id(), &mut tx).await);
        })
        .detach();

        Ok(())
    }

    fn set_posix_lock<F>(
        &self,
        fs: &AsyncRc<F>,
        req: Request,
        ino: Ino,
        body: LockIn,
        block: bool,
    ) -> Result<()>
    where
        F: Filesystem,
    {
        let kind = match body.lk.typ {
            LockOp::UNLOCK => LockKind::Unlock,
            LockOp::READ => LockKind::Read,
            LockOp::WRITE => LockKind::Write,
            _ => return Err(Error::EINVAL),
        };

        if body.lk.start > body.lk.end {
            return Err(Error::EINVAL);
        }

        let fs = fs.clone();
        let mut tx = self.tx.clone();

        spawn(async move {
            let req = SetLockReq {
                req,
                ino,
                fh: FileHandle(body.fh),
                owner: LockOwner(body.owner),
                pid: Pid::from_raw(body.lk.pid.cast_signed()),
                range: FileRange::new(body.lk.start, body.lk.end),
                kind,
                block,
            };

            handle_error(send_result(fs.setlock(&req).await, req.id(), &mut tx).await);
        })
        .detach();

        Ok(())
    }
}
